"""Serve images/CSS from disk with ``static_files`` (WhiteNoise-style).

Place assets under ``static/`` (or any root) and mount the middleware::

    app.use(static_files(root="static", prefix="/static"))

Then HTML can use ``<img src="/static/logo.png">`` without a custom route.

For files next to templates with root-relative URLs::

    app.use(static_files(root="templates/home", prefix="/", fallthrough=True))
"""

from pathlib import Path

from fusion_framework import static_files, status
from fusion_framework.api import FusionBaseApi
from fusion_framework.app import FusionApp
from fusion_framework.config import get_settings, load_settings_module
from fusion_framework.route import route
from fusion_framework.template import FusionBaseTemplate

STATIC_DIR = Path(__file__).resolve().parent / "static_files_assets"


@route("/")
class HomePage(FusionBaseTemplate):
    template = "home/index.html"

    def context(self):
        return {"title": "static files demo"}


@route("/api/ping")
class Ping(FusionBaseApi):
    def get(self):
        return self.response({"ok": True}, status=status.HTTP_SUCCESS)


def main() -> None:
    # Demo assets live beside this example; in apps use project ``static/``.
    STATIC_DIR.mkdir(exist_ok=True)
    logo = STATIC_DIR / "logo.png"
    if not logo.is_file():
        # Minimal valid-looking PNG header bytes for local demos.
        logo.write_bytes(b"\x89PNG\r\n\x1a\n")

    load_settings_module("settings")
    app = FusionApp(get_settings())
    app.use(static_files(root=STATIC_DIR, prefix="/static"))
    app.listen()


if __name__ == "__main__":
    main()
