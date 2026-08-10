from __future__ import annotations

from typing import Any


class HTTPException(Exception):
    """Raise inside a handler (or let binding raise it) to return an HTTP error.

    Example::

        raise HTTPException(400, {"message": "undefined id"})
        raise HTTPException(404, "not found")
    """

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
        body = self.detail
        headers = dict(self.headers)
        if not isinstance(body, (str, bytes)) and body is not None:
            headers = {"content-type": "application/json", **headers}
        response: dict[str, Any] = {"status": self.status, "body": body}
        if headers:
            response["headers"] = headers
        return response
