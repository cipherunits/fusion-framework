"""Preview Fusion home + component gallery + monitor.

    .venv/bin/python examples/preview_templates.py

Then open:
  http://127.0.0.1:3456/                  home (live cache + logs)
  http://127.0.0.1:3456/__fusion/component
  http://127.0.0.1:3456/__fusion/monitor
  http://127.0.0.1:3456/swagger
"""

from __future__ import annotations

import tempfile
from pathlib import Path

from fusion_framework import cache, static_files, tasks
from fusion_framework.api import FusionBaseApi
from fusion_framework.app import FusionApp
from fusion_framework.config import settings
from fusion_framework.route import route
from fusion_framework.template import render_template

REPO_ROOT = Path(__file__).resolve().parents[1]
FUSION_UI = REPO_ROOT / "crates" / "fusion-core" / "assets" / "templates" / "fusion"
# Empty root → only built-in fusion/* templates (avoids parsing static components.html as Tera).
TEMPLATES_ROOT = Path(tempfile.gettempdir()) / "fusion_preview_templates_empty"
TEMPLATES_ROOT.mkdir(parents=True, exist_ok=True)
COMPONENT_PATH = "/__fusion/component"

settings.configure(
    swagger={"enabled": True, "path": "/swagger"},
    monitor={"enabled": True, "path": "/__fusion/monitor"},
    cache={"driver": "moka", "max_events": 50},
    templates={"dir": str(TEMPLATES_ROOT)},
)


@route("/")
class HomePage(FusionBaseApi):
    """Serve the Fusion home page with live cache database + event logs."""

    def get(self):
        """Render fusion/index.html with the real monitor panel context."""
        # mount_monitor re-configures cache on listen and wipes pre-listen seeds;
        # populate a small demo dataset on first empty view so tables stay real.
        ctx = dict(cache.panel_context())
        if ctx.get("empty_entries"):
            cache.set("demo:user", {"name": "Ada"}, ttl=60)
            cache.set("session:1", {"ok": True}, ttl=300)
            cache.set("feature:flag", "on")
            ctx = dict(cache.panel_context())
        ctx["title"] = "Fusion"
        html = render_template(
            "fusion/index.html",
            ctx,
            templates_root=TEMPLATES_ROOT,
        )
        return {
            "status": 200,
            "headers": {"content-type": "text/html; charset=utf-8"},
            "body": html,
        }


def main() -> None:
    if not (FUSION_UI / "index.html").is_file():
        raise SystemExit(f"Home template not found: {FUSION_UI / 'index.html'}")

    cache.configure(settings)
    cache.set("demo:user", {"name": "Ada"}, ttl=60)
    cache.set("session:1", {"ok": True}, ttl=300)
    cache.set("feature:flag", "on")
    tasks.reset()
    tasks.spawn(lambda: cache.set("from:task", True))
    tasks.spawn_after(5_000, lambda: None)

    print("Preview:", flush=True)
    print("  http://127.0.0.1:3456/", flush=True)
    print(f"  http://127.0.0.1:3456{COMPONENT_PATH}", flush=True)
    print("  http://127.0.0.1:3456/__fusion/monitor", flush=True)
    print("  http://127.0.0.1:3456/swagger", flush=True)

    app = FusionApp(settings)
    # Gallery CSS/JS resolve via <base href="/__fusion/component/"> in components.html
    app.use(static_files(root=FUSION_UI, prefix=COMPONENT_PATH, max_age=0))
    app.listen(host="127.0.0.1", port=3456)


if __name__ == "__main__":
    main()
