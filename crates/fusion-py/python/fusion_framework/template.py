"""Tera HTML templates with built-in Fusion UI components."""

from __future__ import annotations

from pathlib import Path
from typing import Any, ClassVar, Mapping, Optional

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
    directory. Override ``context()`` to pass variables. Call ``render()`` from
    a route handler, or rely on the default ``get()`` implementation.

    Built-in UI components are defined in ``fusion/macros.html`` (Tera 2 components)::

        {{<fusion.button label="Save" variant="primary" />}}
    """

    template: ClassVar[str] = ""
    template_address: ClassVar[str] = ""
    templates_dir: ClassVar[str] = ""

    def context(self) -> dict[str, Any]:
        """Template variables (override in subclasses)."""
        return {}

    def get(self) -> dict[str, Any]:
        """Default GET handler — renders the configured template."""
        return self.render()

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
    ) -> dict[str, Any]:
        """Render template and return an HTML response envelope."""
        ctx = dict(self.context())
        if context:
            ctx.update(context)
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
