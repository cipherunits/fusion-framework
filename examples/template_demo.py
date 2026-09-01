"""Template demo.

Install the local package once (from repo root, with venv active)::

    pip install maturin
    maturin develop --manifest-path crates/fusion-py/Cargo.toml

Or::

    ./scripts/dev-install-python.sh

Run from anywhere::

    python examples/template_demo.py

Then open http://127.0.0.1:3000/pages/home

JSON context (same route, content negotiation)::

    curl -H "Accept: application/json" http://127.0.0.1:3000/pages/home
    curl "http://127.0.0.1:3000/pages/home?format=json"
"""

from __future__ import annotations

from pathlib import Path

from fusion_framework import settings
from fusion_framework.app import FusionApp
from fusion_framework.route import route
from fusion_framework.template import FusionBaseTemplate

DEMO_DIR = Path(__file__).resolve().parent

settings.configure(templates={"dir": str(DEMO_DIR / "templates")})


@route("/pages/[module]")
class HomeModule(FusionBaseTemplate):
    template = "home/index.html"

    def context(self):
        return {
            "title": "Fusion Templates",
            "message": "Hello from Tera!",
        }


def main() -> None:
    app = FusionApp()
    app.listen()


if __name__ == "__main__":
    main()
