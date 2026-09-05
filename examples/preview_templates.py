"""Preview the Fusion UI component gallery (index.html) at /ui.

    .venv/bin/python examples/preview_templates.py

Then open:
  http://127.0.0.1:3456/ui
"""

from __future__ import annotations

from pathlib import Path

from fusion_framework import static_files
from fusion_framework.api import FusionBaseApi
from fusion_framework.app import FusionApp
from fusion_framework.config import settings
from fusion_framework.route import route

# Built-in gallery: crates/fusion-core/assets/templates/fusion/index.html
REPO_ROOT = Path(__file__).resolve().parents[1]
FUSION_UI = REPO_ROOT / "crates" / "fusion-core" / "assets" / "templates" / "fusion"
INDEX_HTML = FUSION_UI / "index.html"

settings.configure(
    monitor={"enabled": False},
)


@route("/ui")
class UiGallery(FusionBaseApi):
    """Serve the component gallery HTML at /ui (assets under /ui/…)."""

    def get(self):
        """Return index.html as a text/html response."""
        return {
            "status": 200,
            "headers": {"content-type": "text/html; charset=utf-8"},
            "body": INDEX_HTML.read_bytes(),
        }


def main() -> None:
    if not INDEX_HTML.is_file():
        raise SystemExit(f"Gallery not found: {INDEX_HTML}")

    print("Preview:", flush=True)
    print("  http://127.0.0.1:3456/ui", flush=True)

    app = FusionApp(settings)
    # CSS/JS/components resolve via <base href="/ui/"> in index.html
    app.use(static_files(root=FUSION_UI, prefix="/ui", max_age=0))
    app.listen(host="127.0.0.1", port=3456)


if __name__ == "__main__":
    main()
