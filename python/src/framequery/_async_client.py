from __future__ import annotations

import asyncio
import os
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
from ._errors import FrameQueryError, JobFailedError
from ._models import (
    Job,
    JobPage,
    ProcessingResult,
    Quota,
    _parse_job,
    _parse_quota,
    _parse_result,
)


class AsyncFrameQuery:
    """Async version of FrameQuery. Same API, all methods are awaitable.

    ::

        async with AsyncFrameQuery(api_key="fq_...") as fq:
            result = await fq.process("video.mp4")
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
        self._client = httpx.AsyncClient(
            timeout=timeout,
            headers=build_headers(resolved_key, USER_AGENT),
        )

    async def process(
        self,
        file: Union[str, Path, BinaryIO],
        *,
        filename: Optional[str] = None,
        poll_interval: float = DEFAULT_POLL_INTERVAL,
        timeout: float = DEFAULT_TIMEOUT,
        on_progress: Optional[Callable[[Job], None]] = None,
    ) -> ProcessingResult:
        """Upload a video and poll until done."""
        job = await self.upload(file, filename=filename)
        return await self._poll(job.id, poll_interval, timeout, on_progress)

    async def process_url(
        self,
        url: str,
        *,
        filename: Optional[str] = None,
        poll_interval: float = DEFAULT_POLL_INTERVAL,
        timeout: float = DEFAULT_TIMEOUT,
        on_progress: Optional[Callable[[Job], None]] = None,
    ) -> ProcessingResult:
        """Like ``process()`` but takes a public URL instead of a local file."""
        body: Dict[str, str] = {"url": url}
        if filename:
            body["fileName"] = filename
        data = await self._request("POST", "/jobs/from-url", json=body)
        job = _parse_job(data)
        return await self._poll(job.id, poll_interval, timeout, on_progress)

    async def upload(
        self,
        file: Union[str, Path, BinaryIO],
        *,
        filename: Optional[str] = None,
    ) -> Job:
        """Upload a video and return the Job without polling."""
        if isinstance(file, (str, Path)):
            path = Path(file)
            if not path.is_file():
                raise FileNotFoundError(f"File not found: {path}")
            name = filename or path.name
            data = await self._request("POST", "/jobs", json={"fileName": name})
            upload_url = data["uploadUrl"]
            file_bytes = path.read_bytes()
            await self._upload_to_signed_url(upload_url, file_bytes)
        else:
            name = filename or "video.mp4"
            data = await self._request("POST", "/jobs", json={"fileName": name})
            upload_url = data["uploadUrl"]
            content = file.read() if hasattr(file, "read") else file
            await self._upload_to_signed_url(upload_url, content)

        return _parse_job(data)

    async def get_job(self, job_id: str) -> Job:
        data = await self._request("GET", f"/jobs/{job_id}")
        return _parse_job(data)

    async def list_jobs(
        self,
        *,
        limit: int = 20,
        cursor: Optional[str] = None,
        status: Optional[str] = None,
    ) -> JobPage:
        params: Dict[str, Any] = {"limit": limit}
        if cursor:
            params["cursor"] = cursor
        if status:
            params["status"] = status
        raw = await self._request_raw("GET", "/jobs", params=params)
        items = raw.get("data", [])
        jobs = [_parse_job(j) for j in items]
        return JobPage(jobs=jobs, next_cursor=raw.get("nextCursor"))

    async def get_quota(self) -> Quota:
        data = await self._request("GET", "/quota")
        return _parse_quota(data)

    async def close(self) -> None:
        await self._client.aclose()

    async def __aenter__(self) -> "AsyncFrameQuery":
        return self

    async def __aexit__(self, *args: Any) -> None:
        await self.close()

    # ---- Private ----

    async def _request(self, method: str, path: str, **kwargs: Any) -> Any:
        resp = await self._do_request(method, path, **kwargs)
        return handle_response(resp)

    async def _request_raw(self, method: str, path: str, **kwargs: Any) -> Dict[str, Any]:
        resp = await self._do_request(method, path, **kwargs)
        if not resp.is_success:
            handle_response(resp)
        return resp.json()  # type: ignore[no-any-return]

    async def _do_request(self, method: str, path: str, **kwargs: Any) -> httpx.Response:
        url = f"{self._base_url}{path}"
        last_exc: Optional[Exception] = None

        for attempt in range(self._max_retries + 1):
            try:
                resp = await self._client.request(method, url, **kwargs)
                if resp.status_code < 500 and resp.status_code != 429:
                    return resp
                if attempt < self._max_retries:
                    delay = _backoff_delay(attempt, resp)
                    await asyncio.sleep(delay)
                    continue
                return resp
            except httpx.TransportError as exc:
                last_exc = exc
                if attempt < self._max_retries:
                    await asyncio.sleep(_backoff_delay(attempt))
                    continue
                raise FrameQueryError(f"Request failed after retries: {exc}") from exc

        if last_exc:
            raise FrameQueryError(f"Request failed: {last_exc}") from last_exc
        raise FrameQueryError("Request failed")

    async def _upload_to_signed_url(self, url: str, file_data: Any) -> None:
        resp = await self._client.put(
            url,
            content=file_data,
            headers={"Content-Type": "application/octet-stream"},
        )
        if not resp.is_success:
            raise FrameQueryError(
                f"Upload to signed URL failed with status {resp.status_code}"
            )

    async def _poll(
        self,
        job_id: str,
        poll_interval: float,
        timeout: float,
        on_progress: Optional[Callable[[Job], None]],
    ) -> ProcessingResult:
        import time

        deadline = time.time() + timeout
        interval = poll_interval

        while True:
            job = await self.get_job(job_id)

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

            if job.eta_seconds and job.eta_seconds > 60:
                interval = min(job.eta_seconds / 3, 30.0)
            else:
                interval = poll_interval

            await asyncio.sleep(interval)


def _backoff_delay(attempt: int, response: Optional[httpx.Response] = None) -> float:
    if response is not None:
        ra = response.headers.get("Retry-After")
        if ra:
            try:
                return float(ra)
            except ValueError:
                pass
    return float(min(0.5 * (2**attempt), 30.0))
