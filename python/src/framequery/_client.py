from __future__ import annotations

import os
import time
from io import IOBase
from pathlib import Path
from typing import Any, BinaryIO, Callable, Dict, Optional, Union

import httpx

from ._base_client import build_headers, handle_response
from ._constants import (
    DEFAULT_BASE_URL,
    DEFAULT_HTTP_TIMEOUT,
    DEFAULT_MAX_RETRIES,
    DEFAULT_POLL_INTERVAL,
    DEFAULT_TIMEOUT,
    USER_AGENT,
)
from ._errors import FrameQueryError, JobFailedError, RateLimitError
from ._models import (
    Job,
    JobPage,
    ProcessingResult,
    Quota,
    _parse_job,
    _parse_quota,
    _parse_result,
)


class FrameQuery:
    """Synchronous client for the FrameQuery video processing API.

    Usage::

        from framequery import FrameQuery

        fq = FrameQuery(api_key="fq_...")
        result = fq.process("video.mp4")
        print(result.scenes)
    """

    def __init__(
        self,
        api_key: Optional[str] = None,
        base_url: str = DEFAULT_BASE_URL,
        timeout: float = DEFAULT_HTTP_TIMEOUT,
        max_retries: int = DEFAULT_MAX_RETRIES,
    ) -> None:
        resolved_key = api_key or os.environ.get("FRAMEQUERY_API_KEY", "")
        if not resolved_key:
            raise ValueError(
                "api_key is required. Pass it explicitly or set FRAMEQUERY_API_KEY."
            )

        self._api_key = resolved_key
        self._base_url = base_url.rstrip("/")
        self._max_retries = max_retries
        self._client = httpx.Client(
            timeout=timeout,
            headers=build_headers(resolved_key, USER_AGENT),
        )

    def process(
        self,
        file: Union[str, Path, BinaryIO],
        *,
        filename: Optional[str] = None,
        poll_interval: float = DEFAULT_POLL_INTERVAL,
        timeout: float = DEFAULT_TIMEOUT,
        on_progress: Optional[Callable[[Job], None]] = None,
    ) -> ProcessingResult:
        """Upload a video file and wait for processing to complete.

        Args:
            file: Local file path (str/Path) or a readable binary file object.
            filename: Object name in the ingest bucket. Defaults to the file's name.
            poll_interval: Seconds between status polls. Default 5.
            timeout: Maximum seconds to wait for completion. Default 24h.
            on_progress: Optional callback invoked on each poll with the current Job.

        Returns:
            ProcessingResult with scenes, transcript, and duration.
        """
        job = self.upload(file, filename=filename)
        return self._poll(job.id, poll_interval, timeout, on_progress)

    def process_url(
        self,
        url: str,
        *,
        filename: Optional[str] = None,
        poll_interval: float = DEFAULT_POLL_INTERVAL,
        timeout: float = DEFAULT_TIMEOUT,
        on_progress: Optional[Callable[[Job], None]] = None,
    ) -> ProcessingResult:
        """Submit a URL for processing and wait for completion.

        Args:
            url: Public HTTP(S) URL of the video to process.
            filename: Optional filename hint.
            poll_interval: Seconds between status polls.
            timeout: Maximum seconds to wait.
            on_progress: Optional progress callback.

        Returns:
            ProcessingResult with scenes, transcript, and duration.
        """
        body: Dict[str, str] = {"url": url}
        if filename:
            body["fileName"] = filename
        data = self._request("POST", "/jobs/from-url", json=body)
        job = _parse_job(data)
        return self._poll(job.id, poll_interval, timeout, on_progress)

    def upload(
        self,
        file: Union[str, Path, BinaryIO],
        *,
        filename: Optional[str] = None,
    ) -> Job:
        """Upload a video and return the Job immediately (does not wait).

        Args:
            file: Local file path or binary file object.
            filename: Object name override.

        Returns:
            Job with id and status.
        """
        if isinstance(file, (str, Path)):
            path = Path(file)
            if not path.is_file():
                raise FileNotFoundError(f"File not found: {path}")
            name = filename or path.name
            data = self._request("POST", "/jobs", json={"fileName": name})
            upload_url = data["uploadUrl"]
            with path.open("rb") as fh:
                self._upload_to_signed_url(upload_url, fh)
        else:
            name = filename or "video.mp4"
            data = self._request("POST", "/jobs", json={"fileName": name})
            upload_url = data["uploadUrl"]
            self._upload_to_signed_url(upload_url, file)

        return _parse_job(data)

    def get_job(self, job_id: str) -> Job:
        """Fetch the current state of a job."""
        data = self._request("GET", f"/jobs/{job_id}")
        return _parse_job(data)

    def list_jobs(
        self,
        *,
        limit: int = 20,
        cursor: Optional[str] = None,
        status: Optional[str] = None,
    ) -> JobPage:
        """List jobs with optional filtering and pagination."""
        params: Dict[str, Any] = {"limit": limit}
        if cursor:
            params["cursor"] = cursor
        if status:
            params["status"] = status
        raw = self._request_raw("GET", "/jobs", params=params)
        items = raw.get("data", [])
        jobs = [_parse_job(j) for j in items]
        return JobPage(jobs=jobs, next_cursor=raw.get("nextCursor"))

    def get_quota(self) -> Quota:
        """Get the current account quota."""
        data = self._request("GET", "/quota")
        return _parse_quota(data)

    def close(self) -> None:
        """Close the underlying HTTP client."""
        self._client.close()

    def __enter__(self) -> "FrameQuery":
        return self

    def __exit__(self, *args: Any) -> None:
        self.close()

    # ---- Private ----

    def _request(self, method: str, path: str, **kwargs: Any) -> Any:
        """Make an API request with retry logic. Returns unwrapped data."""
        resp = self._do_request(method, path, **kwargs)
        return handle_response(resp)

    def _request_raw(self, method: str, path: str, **kwargs: Any) -> Dict[str, Any]:
        """Make an API request, return the raw JSON body (not unwrapped)."""
        resp = self._do_request(method, path, **kwargs)
        if not resp.is_success:
            handle_response(resp)  # raises
        return resp.json()  # type: ignore[no-any-return]

    def _do_request(self, method: str, path: str, **kwargs: Any) -> httpx.Response:
        """Execute request with retries on transient errors."""
        url = f"{self._base_url}{path}"
        last_exc: Optional[Exception] = None

        for attempt in range(self._max_retries + 1):
            try:
                resp = self._client.request(method, url, **kwargs)
                if resp.status_code < 500 and resp.status_code != 429:
                    return resp
                if attempt < self._max_retries:
                    delay = _backoff_delay(attempt, resp)
                    time.sleep(delay)
                    last_exc = None
                    continue
                return resp
            except httpx.TransportError as exc:
                last_exc = exc
                if attempt < self._max_retries:
                    time.sleep(_backoff_delay(attempt))
                    continue
                raise FrameQueryError(f"Request failed after retries: {exc}") from exc

        if last_exc:
            raise FrameQueryError(f"Request failed: {last_exc}") from last_exc
        raise FrameQueryError("Request failed")  # unreachable

    def _upload_to_signed_url(self, url: str, file_data: Any) -> None:
        """PUT file bytes to a signed GCS URL."""
        resp = self._client.put(
            url,
            content=file_data if isinstance(file_data, (bytes, bytearray)) else file_data,
            headers={"Content-Type": "application/octet-stream"},
        )
        if not resp.is_success:
            raise FrameQueryError(
                f"Upload to signed URL failed with status {resp.status_code}"
            )

    def _poll(
        self,
        job_id: str,
        poll_interval: float,
        timeout: float,
        on_progress: Optional[Callable[[Job], None]],
    ) -> ProcessingResult:
        """Poll a job until it reaches a terminal state."""
        deadline = time.time() + timeout
        interval = poll_interval

        while True:
            job = self.get_job(job_id)

            if on_progress:
                on_progress(job)

            if job.is_failed:
                error_msg = job.raw.get("errorMessage", "")
                raise JobFailedError(job_id, str(error_msg))

            if job.is_complete:
                return _parse_result(job.raw)

            if time.time() > deadline:
                raise TimeoutError(
                    f"Timed out after {timeout}s waiting for job {job_id}"
                )

            # Adaptive polling: slow down for long-running jobs
            if job.eta_seconds and job.eta_seconds > 60:
                interval = min(job.eta_seconds / 3, 30.0)
            else:
                interval = poll_interval

            time.sleep(interval)


def _backoff_delay(attempt: int, response: Optional[httpx.Response] = None) -> float:
    """Exponential backoff: 0.5s, 1s, 2s, ..."""
    if response is not None:
        ra = response.headers.get("Retry-After")
        if ra:
            try:
                return float(ra)
            except ValueError:
                pass
    return min(0.5 * (2**attempt), 30.0)
