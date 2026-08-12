from __future__ import annotations

from fusion_framework._fusion import App
from fusion_framework.config import get_settings, load_settings_module, settings as settings_store
from fusion_framework._fusion import openapi_spec as _openapi_spec


class FusionApp:
    """Thin façade over the Rust ``App`` engine."""

    def __init__(self, app_settings=None):
        self.settings = app_settings or get_settings()
        self._engine = App()
        self._mounted = False

    def listen(self, host: str | None = None, port: int | None = None) -> None:
        if not self._mounted:
            self._engine.mount_routes()
            swagger_path = self.settings.get("swagger.path", "/swagger")
            if swagger_path:
                prefix = str(swagger_path).rstrip("/")
                if not prefix.startswith("/"):
                    prefix = f"/{prefix}"

                openapi = _openapi_spec()

                def openapi_handler(_req: dict):
                    return openapi

                def ui_handler(_req: dict):
                    openapi_url = f"{prefix}/openapi.json"
                    html = f"""<!doctype html>
<html>
  <head>
    <meta charset="utf-8" />
    <title>Fusion Swagger</title>
    <link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist/swagger-ui.css" />
  </head>
  <body>
    <div id="swagger-ui"></div>
    <script src="https://unpkg.com/swagger-ui-dist/swagger-ui-bundle.js"></script>
    <script>
      window.onload = function() {{
        SwaggerUIBundle({{
          url: '{openapi_url}',
          dom_id: '#swagger-ui',
        }});
      }};
    </script>
  </body>
</html>"""
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
