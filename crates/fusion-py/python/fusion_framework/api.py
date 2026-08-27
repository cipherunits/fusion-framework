from __future__ import annotations

from typing import Any, Mapping, Optional

from fusion_framework._fusion import HTTP_METHODS
from fusion_framework.pagination import PaginationParams, paginated_body, parse_pagination


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

    def response(
        self,
        body: Any = "",
        status: int = 200,
        headers: Mapping[str, str] | None = None,
        **extra: str,
    ) -> dict[str, Any]:
        """Build an HTTP envelope: ``{\"status\": ..., \"body\": ..., \"headers\": ...}``.

        Prefer ``headers=`` (or ``headers=header.download(...)``) for names with hyphens.
        """
        response: dict[str, Any] = {"status": status, "body": body}
        hdrs: dict[str, str] = {**(headers or {}), **extra}
        if not isinstance(body, (str, bytes)) and body is not None:
            hdrs = {"content-type": "application/json", **hdrs}
        if hdrs:
            response["headers"] = hdrs
        return response

    def pagination(
        self,
        *,
        page: Optional[int] = None,
        page_size: Optional[int] = None,
        offset: Optional[int] = None,
        default_page_size: int = 20,
        max_page_size: int = 100,
    ) -> PaginationParams:
        """Read pagination from query (handler kwargs override when passed)."""
        return parse_pagination(
            self.query,
            page=page,
            page_size=page_size,
            offset=offset,
            default_page_size=default_page_size,
            max_page_size=max_page_size,
        )

    def paginated(
        self,
        items: Any,
        total: int,
        params: Optional[PaginationParams] = None,
        *,
        page: Optional[int] = None,
        page_size: Optional[int] = None,
        status: int = 200,
        headers: Mapping[str, str] | None = None,
        **extra: str,
    ) -> dict[str, Any]:
        """Return a paginated list response envelope."""
        if params is None:
            params = self.pagination(page=page, page_size=page_size)
        body = paginated_body(items, total, params)
        return self.response(body, status=status, headers=headers, **extra)


__all__ = ["FusionBaseApi"]
