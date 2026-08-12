from __future__ import annotations

import json
from typing import Any

from fusion_framework._fusion import App
from fusion_framework.config import get_settings, load_settings_module, settings as settings_store
from fusion_framework._fusion import openapi_spec as _openapi_spec


def _as_dict(value: Any) -> dict:
    return value if isinstance(value, dict) else {}


def _swagger_settings(settings) -> dict[str, Any]:
    """Read swagger.* from fusion.<env>.json (with safe defaults)."""
    enabled = settings.get("swagger.enabled", default=True)
    if enabled in (False, 0, "false", "False", "0", "off", "no"):
        return {"enabled": False}

    path = settings.get("swagger.path", default="/swagger")
    if path in (None, False, "", "false", "False", "0", "off", "no"):
        return {"enabled": False}

    prefix = str(path).rstrip("/") or "/swagger"
    if not prefix.startswith("/"):
        prefix = f"/{prefix}"

    info = _as_dict(settings.get("swagger.info", default={}))
    # Allow flat shortcuts: swagger.title / swagger.version / swagger.description
    for key in ("title", "version", "description"):
        flat = settings.get(f"swagger.{key}", default=None)
        if flat is not None and key not in info:
            info[key] = flat

    info.setdefault("title", "fusion-framework")
    info.setdefault("version", "1.0.0")

    page_title = settings.get("swagger.title", default=None) or info.get("title") or "Fusion API Docs"
    ui = {
        "deepLinking": True,
        "displayOperationId": False,
        "defaultModelsExpandDepth": 1,
        "docExpansion": "list",
        "filter": True,
        "tryItOutEnabled": True,
        "persistAuthorization": False,
        "displayRequestDuration": True,
    }
    ui.update(_as_dict(settings.get("swagger.ui", default={})))

    return {
        "enabled": True,
        "path": prefix,
        "page_title": str(page_title),
        "info": info,
        "ui": ui,
    }


def _build_openapi(swagger: dict[str, Any]) -> dict:
    openapi = _openapi_spec()
    if not isinstance(openapi, dict):
        return openapi
    current = _as_dict(openapi.get("info"))
    current.update(swagger["info"])
    openapi["info"] = current
    return openapi


def _swagger_ui_html(swagger: dict[str, Any], openapi_url: str) -> str:
    ui_opts = dict(swagger["ui"])
    ui_opts["url"] = openapi_url
    ui_opts["dom_id"] = "#swagger-ui"
    # Keep JSON safe inside a <script> block.
    ui_json = json.dumps(ui_opts, ensure_ascii=False).replace("</", "<\\/")
    title = json.dumps(swagger["page_title"], ensure_ascii=False)[1:-1]
    return f"""<!doctype html>
<html>
  <head>
    <meta charset="utf-8" />
    <title>{title}</title>
    <link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist/swagger-ui.css" />
  </head>
  <body>
    <div id="swagger-ui"></div>
    <script src="https://unpkg.com/swagger-ui-dist/swagger-ui-bundle.js"></script>
    <script>
      window.onload = function() {{
        SwaggerUIBundle({ui_json});
      }};
    </script>
  </body>
</html>"""


class FusionApp:
    """Thin façade over the Rust ``App`` engine."""

    def __init__(self, app_settings=None):
        self.settings = app_settings or get_settings()
        self._engine = App()
        self._mounted = False

    def listen(self, host: str | None = None, port: int | None = None) -> None:
        if not self._mounted:
            self._engine.mount_routes()
            swagger = _swagger_settings(self.settings)
            if swagger.get("enabled"):
                prefix = swagger["path"]
                openapi = _build_openapi(swagger)

                def openapi_handler(_req: dict):
                    return openapi

                def ui_handler(_req: dict):
                    html = _swagger_ui_html(swagger, f"{prefix}/openapi.json")
                    return {"status": 200, "body": html, "headers": {"content-type": "text/html"}}

                self._engine.route("GET", f"{prefix}/openapi.json", openapi_handler)
                self._engine.route("GET", prefix, ui_handler)
            self._mounted = True
        host = host if host is not None else self.settings.host
        port = port if port is not None else self.settings.port
        if self.settings.debug:
            print(f"fusion listening on http://{host}:{port}", flush=True)
        try:
            self._engine.listen(host, int(port))
        except KeyboardInterrupt:
            print("fusion: stopped", flush=True)


def run(settings_module: str | None = "settings") -> None:
    if settings_module:
        load_settings_module(settings_module)
    else:
        settings_store.load_json()
    FusionApp(get_settings()).listen()
