"""Built-in middleware + per-route header add/delete.

Uses ``examples/fusion.dev.json`` (``middleware.security``, ``request_id``, …).

Run::

    cd examples && python security_headers_demo.py
"""

from pathlib import Path

from fusion_framework import header, status
from fusion_framework.api import FusionBaseApi
from fusion_framework.app import FusionApp
from fusion_framework.middleware import default_builtin_middleware
from fusion_framework.config import get_settings, load_settings_module
from fusion_framework.middleware import add_header, delete_header
from fusion_framework.route import route

HERE = Path(__file__).resolve().parent


@route("/api/[module]", tags=["demo"])
class DemoModule(FusionBaseApi):
    def get(self):
        rid = self.state.get("request_id")
        return self.response(
            {"message": "secure defaults", "request_id": rid},
            status=status.HTTP_SUCCESS,
        )

    @delete_header("X-Powered-By", "X-Framework")
    @add_header(header.cache_control("public, max-age=60"), X_Demo="1")
    def post(self):
        return self.response({"message": "custom headers"}, status=status.HTTP_SUCCESS)

    @add_header(header.LOCATION, "/api/demo")
    def patch(self):
        return self.response({"redirect_hint": True}, status=status.HTTP_SUCCESS)


def main() -> None:
    load_settings_module("settings")
    settings = get_settings()
    settings.ensure_loaded([str(HERE)])
    app = FusionApp(settings)
    for mw in default_builtin_middleware(settings):
        app.use(mw)
    app.listen()


if __name__ == "__main__":
    main()
