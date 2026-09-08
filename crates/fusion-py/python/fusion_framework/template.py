"""Tera HTML templates with built-in Fusion UI components."""

from __future__ import annotations

import inspect
import json
from pathlib import Path
from typing import Any, ClassVar, Mapping, Optional, Union
from urllib.parse import parse_qs

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


def parse_form_body(body: str, content_type: str | None = None) -> dict[str, str]:
    """Parse urlencoded or JSON body into flat string fields."""
    raw = body or ""
    ct = (content_type or "").lower()
    if "application/json" in ct or (raw.lstrip().startswith("{") and "urlencoded" not in ct):
        try:
            data = json.loads(raw) if raw.strip() else {}
        except json.JSONDecodeError:
            data = {}
        if isinstance(data, dict):
            return {str(k): "" if v is None else str(v) for k, v in data.items()}
        return {}
    parsed = parse_qs(raw, keep_blank_values=True)
    return {key: (values[0] if values else "") for key, values in parsed.items()}


class FusionBaseTemplate(FusionBaseApi):
    """Class-based HTML handler using Tera templates.

    Mental model:

    - ``context()`` — **template data** (title, fields, …). Not an HTTP verb.
    - ``get()`` — **HTTP GET**; renders ``context()`` as HTML (or JSON if client wants JSON).
    - ``post()`` — **HTTP POST**; read ``self.form``, validate, return ``ok`` / ``fail``.

    Form helpers (SPA-friendly)::

        def post(self):
            form = self.form
            if not form.get("phone"):
                return self.fail({"phone": "required"}, **form)
            return self.ok(message="saved", **form)

    With ``data-fusion-form`` + ``{% include "fusion/form.js" %}``, the browser
    posts JSON and stays on the same page.

    Template routes are excluded from Swagger/OpenAPI.
    """

    __fusion_template__ = True

    template: ClassVar[str] = ""
    template_address: ClassVar[str] = ""
    templates_dir: ClassVar[str] = ""

    def context(self) -> Union[dict[str, Any], Any]:
        """Template variables (override in subclasses; may be ``async def``)."""
        return {}

    @property
    def form(self) -> dict[str, str]:
        """Parsed POST body (urlencoded or JSON) as flat string fields."""
        content_type = None
        for key, value in self.headers.items():
            if key.lower() == "content-type":
                content_type = str(value)
                break
        return parse_form_body(self.body, content_type)

    def get(self) -> Any:
        """Default GET — HTML page, or ``context()`` JSON when client wants JSON."""
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

    def fail(
        self,
        errors: Mapping[str, str] | None = None,
        message: str | None = None,
        **fields: Any,
    ) -> Any:
        """Validation failure — JSON for SPA fetch, else same template with errors.

        Example::

            return self.fail({"phone": "شماره لازم است"}, message="خطا", **form)
        """
        err = {str(k): str(v) for k, v in dict(errors or {}).items()}
        form_fields = {str(k): "" if v is None else str(v) for k, v in fields.items()}
        payload_message = message or "Validation failed"
        if self.wants_json():
            return self.response(
                {
                    "ok": False,
                    "message": payload_message,
                    "errors": err,
                    "fields": form_fields,
                },
                status=400,
            )
        return self._form_html_result(
            ok=False,
            message=payload_message,
            errors=err,
            fields=form_fields,
            status=400,
        )

    def ok(self, message: str | None = None, **fields: Any) -> Any:
        """Success — JSON for SPA fetch, else same template with ``ok=true``.

        Example::

            return self.ok(message="ثبت شد", **form)
        """
        form_fields = {str(k): "" if v is None else str(v) for k, v in fields.items()}
        payload_message = message or "OK"
        if self.wants_json():
            return self.response(
                {
                    "ok": True,
                    "message": payload_message,
                    "errors": {},
                    "fields": form_fields,
                },
                status=200,
            )
        return self._form_html_result(
            ok=True,
            message=payload_message,
            errors={},
            fields=form_fields,
            status=200,
        )

    def _form_html_result(
        self,
        *,
        ok: bool,
        message: str,
        errors: Mapping[str, str],
        fields: Mapping[str, str],
        status: int,
    ) -> Any:
        """Merge form result into ``context()`` and re-render the same template."""
        raw = self.context()
        if inspect.isawaitable(raw):
            return self._form_html_result_async(
                raw, ok=ok, message=message, errors=errors, fields=fields, status=status
            )
        return self._finish_form_html(
            raw, ok=ok, message=message, errors=errors, fields=fields, status=status
        )

    async def _form_html_result_async(
        self,
        raw: Any,
        *,
        ok: bool,
        message: str,
        errors: Mapping[str, str],
        fields: Mapping[str, str],
        status: int,
    ) -> dict[str, Any]:
        """Await async context then finish form HTML."""
        ctx = await raw
        return self._finish_form_html(
            ctx, ok=ok, message=message, errors=errors, fields=fields, status=status
        )

    def _finish_form_html(
        self,
        ctx: Any,
        *,
        ok: bool,
        message: str,
        errors: Mapping[str, str],
        fields: Mapping[str, str],
        status: int,
    ) -> dict[str, Any]:
        """Apply form result onto context and render HTML."""
        data = dict(ctx or {})
        data.update(fields)
        data["ok"] = ok
        data["message"] = message
        data["errors"] = dict(errors)
        data["fields"] = dict(fields)
        return self._html_response(data, status=status)

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


__all__ = ["FusionBaseTemplate", "render_template", "parse_form_body"]
