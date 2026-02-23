from __future__ import annotations


class FrameQueryError(Exception):
    """Base for all SDK errors."""

    def __init__(self, message: str) -> None:
        self.message = message
        super().__init__(message)


class AuthenticationError(FrameQueryError):
    """HTTP 401 -- bad or missing API key."""


class PermissionDeniedError(FrameQueryError):
    """HTTP 403 -- key lacks required scopes."""


class NotFoundError(FrameQueryError):
    """HTTP 404."""


class RateLimitError(FrameQueryError):
    """HTTP 429. Check ``retry_after`` for the server-suggested wait (seconds)."""

    def __init__(self, message: str, retry_after: float | None = None) -> None:
        super().__init__(message)
        self.retry_after = retry_after


class APIError(FrameQueryError):
    """Catch-all for non-2xx responses not covered above."""

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
    """The job reached FAILED status during polling."""

    def __init__(self, job_id: str, message: str = "") -> None:
        msg = f"Job {job_id} failed"
        if message:
            msg += f": {message}"
        super().__init__(msg)
        self.job_id = job_id
