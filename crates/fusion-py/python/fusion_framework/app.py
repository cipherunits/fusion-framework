from __future__ import annotations

import json
from typing import Any

from fusion_framework._fusion import App
from fusion_framework.config import get_settings, load_settings_module, settings as settings_store
from fusion_framework._fusion import openapi_spec as _openapi_spec
from fusion_framework.middleware import framework_headers, set_active_global


def _as_dict(value: Any) -> dict:
    return value if isinstance(value, dict) else {}


def _as_list(value: Any) -> list:
    return value if isinstance(value, list) else []


def _truthy_enabled(value: Any, default: bool = True) -> bool:
    if value is None:
        return default
    if value in (False, 0, "false", "False", "0", "off", "no"):
        return False
    if value in (True, 1, "true", "True", "1", "on", "yes"):
        return True
    return bool(value)


def _swagger_settings(settings) -> dict[str, Any]:
    """Read swagger.* from fusion.<env>.json (with safe defaults)."""
    if not _truthy_enabled(settings.get("swagger.enabled", default=True)):
        return {"enabled": False}

    path = settings.get("swagger.path", default="/swagger")
    if path in (None, False, "", "false", "False", "0", "off", "no"):
        return {"enabled": False}

    prefix = str(path).rstrip("/") or "/swagger"
    if not prefix.startswith("/"):
        prefix = f"/{prefix}"

    info = _as_dict(settings.get("swagger.info", default={}))
    for key in ("title", "version", "description", "termsOfService"):
        flat = settings.get(f"swagger.{key}", default=None)
        if flat is not None and key not in info:
            info[key] = flat
    for key in ("contact", "license"):
        flat = settings.get(f"swagger.{key}", default=None)
        if flat is not None and key not in info:
            info[key] = flat

    info.setdefault("title", "fusion-framework")
    info.setdefault("version", "1.0.0")

    page_title = settings.get("swagger.title", default=None) or info.get("title") or "Fusion API Docs"

    auth = _as_dict(settings.get("swagger.auth", default={}))
    schemes = _as_dict(auth.get("schemes"))
    oauth = _as_dict(auth.get("oauth"))
    global_security = _as_list(auth.get("global"))
    persist_auth = auth.get("persistAuthorization")
    if persist_auth is None:
        persist_auth = False

    navbar = _as_dict(settings.get("swagger.navbar", default={}))
    navbar_enabled = _truthy_enabled(navbar.get("enabled", True), default=True)
    show_url_input = _truthy_enabled(navbar.get("showUrlInput", True), default=True)
    navbar_urls = navbar.get("urls")
    if not isinstance(navbar_urls, list):
        navbar_urls = None

    ui = {
        "deepLinking": True,
        "displayOperationId": False,
        "defaultModelsExpandDepth": 1,
        "defaultModelExpandDepth": 1,
        "defaultModelRendering": "example",
        "docExpansion": "list",
        "filter": True,
        "tryItOutEnabled": True,
        "persistAuthorization": bool(persist_auth),
        "displayRequestDuration": True,
        "showExtensions": False,
        "showCommonExtensions": False,
        "syntaxHighlight": {"activated": True, "theme": "agate"},
        "withCredentials": False,
        "validatorUrl": "https://validator.swagger.io/validator",
    }
    ui.update(_as_dict(settings.get("swagger.ui", default={})))
    # auth.persistAuthorization wins over ui.persistAuthorization when set under auth.
    if "persistAuthorization" in auth:
        ui["persistAuthorization"] = bool(persist_auth)

    servers = settings.get("swagger.servers", default=None)
    if not isinstance(servers, list):
        servers = []

    return {
        "enabled": True,
        "path": prefix,
        "page_title": str(page_title),
        "info": info,
        "servers": servers,
        "auth": {
            "schemes": schemes,
            "global": global_security,
            "oauth": oauth,
            "persistAuthorization": bool(persist_auth),
        },
        "navbar": {
            "enabled": navbar_enabled,
            "showUrlInput": show_url_input,
            "urls": navbar_urls,
        },
        "ui": ui,
    }


def _build_openapi(swagger: dict[str, Any]) -> dict:
    openapi = _openapi_spec()
    if not isinstance(openapi, dict):
        return openapi

    current = _as_dict(openapi.get("info"))
    current.update(swagger["info"])
    openapi["info"] = current

    if swagger.get("servers"):
        openapi["servers"] = swagger["servers"]

    schemes = _as_dict(swagger.get("auth", {}).get("schemes"))
    if schemes:
        components = _as_dict(openapi.get("components"))
        security_schemes = _as_dict(components.get("securitySchemes"))
        security_schemes.update(schemes)
        components["securitySchemes"] = security_schemes
        openapi["components"] = components

    global_security = swagger.get("auth", {}).get("global") or []
    if global_security:
        openapi["security"] = global_security

    return openapi


def _swagger_ui_html(swagger: dict[str, Any], openapi_url: str) -> str:
    ui_opts = dict(swagger["ui"])
    navbar = swagger["navbar"]
    auth = swagger["auth"]

    # presets/layout/plugins are set in JS so we can reference SwaggerUIBundle globals.
    ui_opts.pop("presets", None)
    ui_opts.pop("plugins", None)
    ui_opts.pop("layout", None)

    if navbar.get("urls"):
        ui_opts.pop("url", None)
        ui_opts["urls"] = navbar["urls"]
    else:
        ui_opts["url"] = openapi_url
        ui_opts.pop("urls", None)

    ui_opts["dom_id"] = "#swagger-ui"

    ui_json = json.dumps(ui_opts, ensure_ascii=False).replace("</", "<\\/")
    oauth = _as_dict(auth.get("oauth"))
    oauth_json = json.dumps(oauth, ensure_ascii=False).replace("</", "<\\/") if oauth else "null"
    title = json.dumps(swagger["page_title"], ensure_ascii=False)[1:-1]

    navbar_enabled = bool(navbar.get("enabled"))
    show_url_input = bool(navbar.get("showUrlInput", True))
    hide_url_css = ""
    if navbar_enabled and not show_url_input:
        hide_url_css = """
    <style>
      .topbar .download-url-wrapper { display: none !important; }
    </style>"""

    standalone_script = ""
    if navbar_enabled:
        standalone_script = (
            '<script src="https://unpkg.com/swagger-ui-dist/swagger-ui-standalone-preset.js"></script>'
        )

    bootstrap = f"""
      window.onload = function() {{
        var opts = {ui_json};
        opts.presets = [SwaggerUIBundle.presets.apis];
        opts.plugins = [SwaggerUIBundle.plugins.DownloadUrl];
        if ({str(navbar_enabled).lower()} && typeof SwaggerUIStandalonePreset !== 'undefined') {{
          opts.presets.push(SwaggerUIStandalonePreset);
          opts.layout = 'StandaloneLayout';
        }} else {{
          opts.layout = 'BaseLayout';
        }}
        var ui = SwaggerUIBundle(opts);
        var oauth = {oauth_json};
        if (oauth && typeof ui.initOAuth === 'function') {{
          ui.initOAuth(oauth);
        }}
        window.ui = ui;
      }};
    """

    return f"""<!doctype html>
<html>
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>{title}</title>
    <link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist/swagger-ui.css" />
    {hide_url_css}
  </head>
  <body>
    <div id="swagger-ui"></div>
    <script src="https://unpkg.com/swagger-ui-dist/swagger-ui-bundle.js"></script>
    {standalone_script}
    <script>
{bootstrap}
    </script>
  </body>
</html>"""


class FusionApp:
    """Thin façade over the Rust ``App`` engine."""

    def __init__(self, app_settings=None):
        self.settings = app_settings or get_settings()
        self._engine = App()
        self._mounted = False
        # Default: advertise Fusion to clients / Wappalyzer-style detectors.
        self._middleware: list = [framework_headers()]

    def use(self, middleware) -> None:
        """Register global middleware: ``(request, call_next) -> response``."""
        self._middleware.append(middleware)

    def listen(self, host: str | None = None, port: int | None = None) -> None:
        if not self._mounted:
            set_active_global(self._middleware)
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


def run(settings_module: str | None = "settings", middleware: list | None = None) -> None:
    """Start the app. For middleware, prefer explicit ``FusionApp`` in ``main.py``."""
    if settings_module:
        load_settings_module(settings_module)
    else:
        settings_store.load_json()
    app = FusionApp(get_settings())
    for mw in middleware or []:
        app.use(mw)
    app.listen()
