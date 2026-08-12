"""Re-export ``HTTPException`` — response building lives in Rust."""

from __future__ import annotations

from typing import Any

from fusion_framework._fusion import http_error_to_response


class HTTPException(Exception):
    """Raise inside a handler to return an HTTP error response."""

    def __init__(self, status: int, detail: Any = None, **headers: str):
        self.status = int(status)
        self.detail = "" if detail is None else detail
        self.headers = headers
        super().__init__(self._message())

    def _message(self) -> str:
        if isinstance(self.detail, str):
            return self.detail or f"HTTP {self.status}"
        return f"HTTP {self.status}"

    def to_response(self) -> dict[str, Any]:
        return http_error_to_response(self.status, self.detail, **self.headers)
