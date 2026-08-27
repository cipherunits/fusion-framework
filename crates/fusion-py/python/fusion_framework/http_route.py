"""Method-level HTTP route decorators (custom paths + verbs)."""

from __future__ import annotations

from typing import Callable, Optional, Sequence, TypeVar

F = TypeVar("F", bound=Callable[..., object])


def _http_route(
    http_method: str,
    route: str,
    *,
    tags: Optional[Sequence[str]] = None,
    desc: Optional[str] = None,
    title: Optional[str] = None,
    deprecated: bool = False,
) -> Callable[[F], F]:
    def decorator(fn: F) -> F:
        # tags=None means inherit from @route(...); tags=[] clears; tags=[...] overrides.
        fn.__fusion_http_route__ = {
            "method": http_method.lower(),
            "template": route,
            "tags": list(tags) if tags is not None else None,
            "desc": desc,
            "title": title,
            "deprecated": deprecated,
        }
        return fn

    return decorator


def http_get(
    route: str,
    *,
    tags: Optional[Sequence[str]] = None,
    desc: Optional[str] = None,
    title: Optional[str] = None,
    deprecated: bool = False,
) -> Callable[[F], F]:
    return _http_route("get", route, tags=tags, desc=desc, title=title, deprecated=deprecated)


def http_post(
    route: str,
    *,
    tags: Optional[Sequence[str]] = None,
    desc: Optional[str] = None,
    title: Optional[str] = None,
    deprecated: bool = False,
) -> Callable[[F], F]:
    return _http_route("post", route, tags=tags, desc=desc, title=title, deprecated=deprecated)


def http_put(
    route: str,
    *,
    tags: Optional[Sequence[str]] = None,
    desc: Optional[str] = None,
    title: Optional[str] = None,
    deprecated: bool = False,
) -> Callable[[F], F]:
    return _http_route("put", route, tags=tags, desc=desc, title=title, deprecated=deprecated)


def http_patch(
    route: str,
    *,
    tags: Optional[Sequence[str]] = None,
    desc: Optional[str] = None,
    title: Optional[str] = None,
    deprecated: bool = False,
) -> Callable[[F], F]:
    return _http_route("patch", route, tags=tags, desc=desc, title=title, deprecated=deprecated)


def http_delete(
    route: str,
    *,
    tags: Optional[Sequence[str]] = None,
    desc: Optional[str] = None,
    title: Optional[str] = None,
    deprecated: bool = False,
) -> Callable[[F], F]:
    return _http_route("delete", route, tags=tags, desc=desc, title=title, deprecated=deprecated)


def http_head(
    route: str,
    *,
    tags: Optional[Sequence[str]] = None,
    desc: Optional[str] = None,
    title: Optional[str] = None,
    deprecated: bool = False,
) -> Callable[[F], F]:
    return _http_route("head", route, tags=tags, desc=desc, title=title, deprecated=deprecated)


def http_options(
    route: str,
    *,
    tags: Optional[Sequence[str]] = None,
    desc: Optional[str] = None,
    title: Optional[str] = None,
    deprecated: bool = False,
) -> Callable[[F], F]:
    return _http_route("options", route, tags=tags, desc=desc, title=title, deprecated=deprecated)


HttpGet = http_get
HttpPost = http_post
HttpPut = http_put
HttpPatch = http_patch
HttpDelete = http_delete
HttpHead = http_head
HttpOptions = http_options
