from __future__ import annotations


class FrameQueryError(Exception):
    """Base exception for all FrameQuery SDK errors."""

    def __init__(self, message: str) -> None:
        self.message = message
        super().__init__(message)


class AuthenticationError(FrameQueryError):
    """Raised when the API key is invalid or missing (HTTP 401)."""


class PermissionDeniedError(FrameQueryError):
    """Raised when the API key lacks required scopes (HTTP 403)."""


class NotFoundError(FrameQueryError):
    """Raised when the requested resource does not exist (HTTP 404)."""


class RateLimitError(FrameQueryError):
    """Raised when the API rate limit is exceeded (HTTP 429)."""

    def __init__(self, message: str, retry_after: float | None = None) -> None:
        super().__init__(message)
        self.retry_after = retry_after


class APIError(FrameQueryError):
    """Raised for unexpected HTTP errors from the API."""

    def __init__(
        self,
        message: str,
        status_code: int,
        body: dict[str, object] | None = None,
    ) -> None:
        super().__init__(message)
        self.status_code = status_code
        self.body = body


class JobFailedError(FrameQueryError):
    """Raised when a polled job reaches FAILED status."""

    def __init__(self, job_id: str, message: str = "") -> None:
        msg = f"Job {job_id} failed"
        if message:
            msg += f": {message}"
        super().__init__(msg)
        self.job_id = job_id
