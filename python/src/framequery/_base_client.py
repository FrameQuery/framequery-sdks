from __future__ import annotations

from typing import Any, Dict

import httpx

from ._errors import (
    APIError,
    AuthenticationError,
    NotFoundError,
    PermissionDeniedError,
    RateLimitError,
)


def build_headers(api_key: str, user_agent: str) -> Dict[str, str]:
    return {
        "Authorization": f"Bearer {api_key}",
        "User-Agent": user_agent,
    }


def handle_response(response: httpx.Response) -> Any:
    """Parse a JSON response, raising typed errors for non-2xx status codes."""
    if response.is_success:
        if not response.content:
            return None
        body = response.json()
        if isinstance(body, dict) and "data" in body:
            return body["data"]
        return body

    status = response.status_code
    message = f"API error {status}"
    body = None
    try:
        body = response.json()
        if isinstance(body, dict):
            msg = body.get("error") or body.get("message")
            if msg:
                message = str(msg)
    except Exception:
        text = response.text
        if text:
            message = text

    if status == 401:
        raise AuthenticationError(message)
    if status == 403:
        raise PermissionDeniedError(message)
    if status == 404:
        raise NotFoundError(message)
    if status == 429:
        retry_after = None
        ra_header = response.headers.get("Retry-After")
        if ra_header:
            try:
                retry_after = float(ra_header)
            except ValueError:
                pass
        raise RateLimitError(message, retry_after=retry_after)

    raise APIError(message, status_code=status, body=body)
