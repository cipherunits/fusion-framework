"""Tera HTML templates with built-in Fusion UI components."""

from __future__ import annotations

import inspect
from pathlib import Path
from typing import Any, ClassVar, Mapping, Optional, Union

from fusion_framework._fusion import render_template as _render_template
from fusion_framework.api import FusionBaseApi
from fusion_framework.config import settings


def render_template(
    template_name: str,
    context: Mapping[str, Any] | None = None,
    *,
    templates_root: str | Path | None = None,
) -> str:
    """Render a Tera template (path relative to the templates root)."""
    root = str(templates_root or settings.get("templates.dir", default="templates"))
    return _render_template(template_name, dict(context or {}), root)


class FusionBaseTemplate(FusionBaseApi):
    """Class-based HTML handler using Tera templates.

    Set ``template`` (or ``template_address``) to the file path under the templates
    directory. Override ``context()`` to pass variables — sync or ``async def``.
    The default ``get()`` renders HTML for browsers and returns ``context()`` as JSON
    when the client sends ``Accept: application/json`` or ``?format=json``.

    Template routes are mounted as HTTP handlers but are excluded from Swagger/OpenAPI.

    Built-in UI components are defined in ``fusion/macros.html`` (Tera 2 components)::

        {{<fusion.button label="Save" href="/save" variant="primary" />}}
        {{<fusion.badge label="Ready" variant="success" dot={true} />}}
        {{<fusion.table headers={cols} rows={rows} />}}

    Include styles with ``{% include "fusion/components.css" %}`` or extend
    ``fusion/base.html``.
    """

    __fusion_template__ = True

    template: ClassVar[str] = ""
    template_address: ClassVar[str] = ""
    templates_dir: ClassVar[str] = ""

    def context(self) -> Union[dict[str, Any], Any]:
        """Template variables (override in subclasses; may be ``async def``)."""
        return {}

    def get(self) -> Any:
        """Default GET — HTML page, or ``context()`` JSON when client wants JSON.

        Supports sync or async ``context()``; async returns an awaitable for the
        framework event loop.
        """
        raw = self.context()
        if inspect.isawaitable(raw):
            return self._get_async(raw)
        return self._finish_get(raw)

    async def _get_async(self, raw: Any) -> Any:
        """Await async ``context()`` then finish the GET response."""
        ctx = await raw
        return self._finish_get(ctx)

    def _finish_get(self, ctx: Any) -> dict[str, Any]:
        """Build JSON or HTML from an already-resolved context mapping."""
        data = dict(ctx or {})
        if self.wants_json():
            return data
        return self._html_response(data)

    def template_name(self) -> str:
        """Resolved template path (override for dynamic templates)."""
        name = self.template or self.template_address
        if not name:
            raise ValueError(
                f"{type(self).__name__} must set `template` or `template_address`"
            )
        return name

    def templates_root(self) -> str:
        """Directory containing template files."""
        if self.templates_dir:
            return self.templates_dir
        return str(settings.get("templates.dir", default="templates"))

    def render(
        self,
        status: int = 200,
        headers: Mapping[str, str] | None = None,
        *,
        context: Mapping[str, Any] | None = None,
        template_name: str | None = None,
        **extra: str,
    ) -> Any:
        """Render template and return an HTML response envelope.

        If ``context()`` is async and ``context=`` is omitted, returns an awaitable.
        """
        raw = self.context()
        if inspect.isawaitable(raw):
            return self._render_async(
                raw,
                status=status,
                headers=headers,
                context=context,
                template_name=template_name,
                **extra,
            )
        ctx = dict(raw or {})
        if context:
            ctx.update(context)
        return self._html_response(
            ctx,
            status=status,
            headers=headers,
            template_name=template_name,
            **extra,
        )

    async def _render_async(
        self,
        raw: Any,
        *,
        status: int = 200,
        headers: Mapping[str, str] | None = None,
        context: Mapping[str, Any] | None = None,
        template_name: str | None = None,
        **extra: str,
    ) -> dict[str, Any]:
        """Await async ``context()`` then render HTML."""
        ctx = dict(await raw)
        if context:
            ctx.update(context)
        return self._html_response(
            ctx,
            status=status,
            headers=headers,
            template_name=template_name,
            **extra,
        )

    def _html_response(
        self,
        ctx: Mapping[str, Any],
        status: int = 200,
        headers: Mapping[str, str] | None = None,
        *,
        template_name: str | None = None,
        **extra: str,
    ) -> dict[str, Any]:
        """Render ``ctx`` into an HTML response envelope."""
        html = render_template(
            template_name or self.template_name(),
            ctx,
            templates_root=self.templates_root(),
        )
        hdrs: dict[str, str] = {"content-type": "text/html; charset=utf-8"}
        if headers:
            hdrs.update(headers)
        hdrs.update(extra)
        return self.response(html, status=status, headers=hdrs)


__all__ = ["FusionBaseTemplate", "render_template"]
