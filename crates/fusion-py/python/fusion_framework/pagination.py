"""Pagination helpers for list APIs."""

from __future__ import annotations

from typing import Any, Mapping, Optional

from fusion_framework._fusion import PaginationParams
from fusion_framework._fusion import paginated_body as _paginated_body
from fusion_framework._fusion import parse_pagination as _parse_pagination


def parse_pagination(
    query: Mapping[str, str],
    *,
    page: Optional[int] = None,
    page_size: Optional[int] = None,
    offset: Optional[int] = None,
    default_page_size: int = 20,
    max_page_size: int = 100,
) -> PaginationParams:
    """Parse ``page``, ``page_size`` / ``per_page`` / ``limit``, and optional ``offset``.

    Handler kwargs override query-string values when provided.
    """
    merged = dict(query)
    if page is not None:
        merged["page"] = str(page)
    if page_size is not None:
        merged["page_size"] = str(page_size)
    if offset is not None:
        merged["offset"] = str(offset)
    return _parse_pagination(
        merged,
        default_page_size=default_page_size,
        max_page_size=max_page_size,
    )


def paginated_body(items: Any, total: int, params: PaginationParams) -> dict[str, Any]:
    """Build ``{ items, pagination }`` JSON body."""
    body = _paginated_body(items, int(total), params)
    if not isinstance(body, dict):
        raise TypeError("paginated_body expected dict from core")
    return body


__all__ = ["PaginationParams", "parse_pagination", "paginated_body"]
