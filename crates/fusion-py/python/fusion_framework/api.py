from __future__ import annotations

from typing import Any, Mapping

from fusion_framework._fusion import HTTP_METHODS


class FusionBaseApi:
    """Host-language request view. Serialization rules live in fusion-core."""

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

    def response(
        self,
        body: Any = "",
        status: int = 200,
        **headers: str,
    ) -> dict[str, Any]:
        """Build an HTTP envelope: ``{"status": ..., "body": ...}``.

        Example::

            return self.response({"message": "hello"}, status=200)
        """
        response: dict[str, Any] = {"status": status, "body": body}
        if not isinstance(body, (str, bytes)) and body is not None:
            headers = {"content-type": "application/json", **headers}
        if headers:
            response["headers"] = headers
        return response
