"""Render Fusion Tera UI components (button, badge, table + page_size).

    python examples/ui_components.py
"""

from __future__ import annotations

from pathlib import Path

from fusion_framework.template import render_template

ROOT = Path(__file__).resolve().parent / "ui_components_assets"


def main() -> None:
    html = render_template(
        "page.html",
        {
            "headers": ["Route", "Method"],
            "rows": [
                ["/v1/api/product", "GET"],
                ["/swagger", "GET"],
                ["/__fusion/cache", "GET"],
                ["/health", "GET"],
            ],
        },
        templates_root=ROOT,
    )
    print(html)


if __name__ == "__main__":
    main()
