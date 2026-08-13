from __future__ import annotations

from typing import Any, Mapping

from fusion_framework._fusion import HTTP_METHODS


class FusionBaseApi:
    """Thin Python interface (subclassable).

    The framework core does:
    - request parsing (method/path/query/body)
    - parameter binding to your handler signature
    - response serialization and JSON encoding
    """

    HTTP_METHODS = HTTP_METHODS

    def __init__(self, request: Mapping[str, Any]):
        self.request = request

    @property
    def method(self) -> str:
        return str(self.request.get("method", "")).upper()

    @property
    def path(self) -> str:
        return str(self.request.get("path", ""))

    @property
    def body(self) -> str:
        return str(self.request.get("body", ""))

    @property
    def headers(self) -> Mapping[str, str]:
        return self.request.get("headers") or {}

    @property
    def params(self) -> Mapping[str, str]:
        return self.request.get("params") or {}

    @property
    def query(self) -> Mapping[str, str]:
        return self.request.get("query") or {}

    @property
    def state(self) -> Mapping[str, Any]:
        return self.request.get("state") or {}

    def response(self, body: Any = "", status: int = 200, **headers: str) -> dict[str, Any]:
        """Build an HTTP envelope: ``{\"status\": ..., \"body\": ...}``."""
        response: dict[str, Any] = {"status": status, "body": body}
        if not isinstance(body, (str, bytes)) and body is not None:
            headers = {"content-type": "application/json", **headers}
        if headers:
            response["headers"] = headers
        return response


__all__ = ["FusionBaseApi"]
