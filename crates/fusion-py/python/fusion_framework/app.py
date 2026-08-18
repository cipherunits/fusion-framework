from __future__ import annotations

import json
from typing import Any

from fusion_framework._fusion import (
    App,
    openapi_spec as _openapi_spec,
    route_versions as _route_versions,
    has_unversioned_routes as _has_unversioned_routes,
)
from fusion_framework.config import get_settings, load_settings_module, settings as settings_store
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
    show_url_input_set = "showUrlInput" in navbar
    show_url_input = _truthy_enabled(navbar.get("showUrlInput", True), default=True)
    navbar_urls = navbar.get("urls")
    urls_set = isinstance(navbar_urls, list)
    if not urls_set:
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
            "showUrlInputSet": show_url_input_set,
            "urls": navbar_urls,
            "urlsSet": urls_set,
        },
        "ui": ui,
    }


UNVERSIONED_SWAGGER_NAME = "default"


def _normalize_version_label(value: Any) -> str:
    return str(value or "").strip().strip("/")


def _swagger_version_urls(prefix: str) -> list[dict[str, str]]:
    urls: list[dict[str, str]] = []
    for version in _route_versions():
        label = _normalize_version_label(version)
        if not label:
            continue
        urls.append({"url": f"{prefix}/{label}/openapi.json", "name": label})
    if _has_unversioned_routes() and urls:
        urls.append(
            {
                "url": f"{prefix}/{UNVERSIONED_SWAGGER_NAME}/openapi.json",
                "name": UNVERSIONED_SWAGGER_NAME,
            }
        )
    return urls


def _apply_version_navbar(swagger: dict[str, Any]) -> list[str]:
    """Fill navbar spec URLs from `@route(version=...)` when the user did not set them."""
    navbar = swagger.setdefault("navbar", {})
    auto_urls = _swagger_version_urls(swagger["path"])
    labels = [item["name"] for item in auto_urls]
    if not navbar.get("urlsSet") and auto_urls:
        navbar["urls"] = auto_urls
        if not navbar.get("showUrlInputSet"):
            navbar["showUrlInput"] = False
    return labels


def _build_openapi(swagger: dict[str, Any], version: str | None = None) -> dict:
    openapi = _openapi_spec() if version is None else _openapi_spec(version)
    if not isinstance(openapi, dict):
        return openapi

    current = _as_dict(openapi.get("info"))
    current.update(swagger["info"])
    label = _normalize_version_label(version or "")
    if label and label != UNVERSIONED_SWAGGER_NAME:
        current["version"] = label
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


def _swagger_ui_html(swagger: dict[str, Any], openapi_url: str, primary_name: str | None = None) -> str:
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
        name = primary_name or (navbar["urls"][0].get("name") if navbar["urls"] else None)
        if name:
            ui_opts["urls.primaryName"] = name
    else:
        ui_opts["url"] = openapi_url
        ui_opts.pop("urls", None)
        ui_opts.pop("urls.primaryName", None)

    ui_opts["dom_id"] = "#swagger-ui"

    ui_json = json.dumps(ui_opts, ensure_ascii=False).replace("</", "<\\/")
    oauth = _as_dict(auth.get("oauth"))
    oauth_json = json.dumps(oauth, ensure_ascii=False).replace("</", "<\\/") if oauth else "null"
    title = json.dumps(swagger["page_title"], ensure_ascii=False)[1:-1]

    navbar_enabled = bool(navbar.get("enabled"))
    show_url_input = bool(navbar.get("showUrlInput", True))
    hide_url_css = ""
    if navbar_enabled and not show_url_input:
        # Keep the version <select>; hide only the free-text Explore box.
        hide_url_css = """
    <style>
      .topbar form { display: none !important; }
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


def _html_response(html: str) -> dict:
    return {"status": 200, "body": html, "headers": {"content-type": "text/html"}}


def _mount_swagger(engine, swagger: dict[str, Any]) -> None:
    prefix = swagger["path"]
    labels = _apply_version_navbar(swagger)

    combined = _build_openapi(swagger)
    engine.route("GET", f"{prefix}/openapi.json", lambda _req, spec=combined: spec)

    for label in labels:
        spec = _build_openapi(swagger, version=label)
        engine.route(
            "GET",
            f"{prefix}/{label}/openapi.json",
            lambda _req, spec=spec: spec,
        )
        engine.route(
            "GET",
            f"{prefix}/{label}",
            lambda _req, name=label: _html_response(
                _swagger_ui_html(swagger, f"{prefix}/{name}/openapi.json", primary_name=name)
            ),
        )
        engine.route(
            "GET",
            f"{prefix}/{label}/",
            lambda _req, name=label: _html_response(
                _swagger_ui_html(swagger, f"{prefix}/{name}/openapi.json", primary_name=name)
            ),
        )

    def ui_root(_req: dict):
        return _html_response(_swagger_ui_html(swagger, f"{prefix}/openapi.json"))

    engine.route("GET", prefix, ui_root)
    if prefix != "/":
        engine.route("GET", f"{prefix}/", ui_root)


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
                _mount_swagger(self._engine, swagger)
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
