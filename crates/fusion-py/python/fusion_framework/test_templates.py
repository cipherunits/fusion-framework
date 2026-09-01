"""Tests for Tera template rendering."""

from __future__ import annotations

import tempfile
from pathlib import Path

import pytest

from fusion_framework.template import FusionBaseTemplate, render_template


def test_render_builtin_button_macro():
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        (root / "page.html").write_text(
            '{{<fusion.button label="Go" href="/" />}}',
            encoding="utf-8",
        )
        html = render_template("page.html", {}, templates_root=root)
        assert "fusion-btn" in html
        assert "Go" in html


def test_fusion_base_template_context():
    class Page(FusionBaseTemplate):
        template = "hello.html"

        def context(self):
            return {"name": "Fusion"}

    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        (root / "hello.html").write_text("<p>Hello {{ name }}!</p>", encoding="utf-8")

        page = Page({"method": "GET", "path": "/"})
        page.templates_dir = str(root)
        out = page.render()
        assert out["status"] == 200
        assert out["headers"]["content-type"].startswith("text/html")
        assert "Hello Fusion!" in out["body"]


def test_template_name_required():
    class Bad(FusionBaseTemplate):
        pass

    with pytest.raises(ValueError, match="template"):
        Bad({}).template_name()


def test_template_get_returns_json_with_accept():
    class Page(FusionBaseTemplate):
        template = "hello.html"

        def context(self):
            return {"name": "Fusion"}

    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        (root / "hello.html").write_text("<p>Hello {{ name }}!</p>", encoding="utf-8")

        page = Page(
            {
                "method": "GET",
                "path": "/pages/home",
                "headers": {"accept": "application/json"},
            }
        )
        page.templates_dir = str(root)
        assert page.get() == {"name": "Fusion"}


def test_template_get_returns_json_with_format_query():
    class Page(FusionBaseTemplate):
        template = "hello.html"

        def context(self):
            return {"name": "Fusion"}

    page = Page(
        {
            "method": "GET",
            "path": "/pages/home",
            "query": {"format": "json"},
        }
    )
    assert page.get() == {"name": "Fusion"}


def test_template_get_returns_html_for_browser_accept():
    class Page(FusionBaseTemplate):
        template = "hello.html"

        def context(self):
            return {"name": "Fusion"}

    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        (root / "hello.html").write_text("<p>Hello {{ name }}!</p>", encoding="utf-8")

        page = Page(
            {
                "method": "GET",
                "path": "/pages/home",
                "headers": {
                    "accept": "text/html,application/xhtml+xml,application/xml;q=0.9"
                },
            }
        )
        page.templates_dir = str(root)
        out = page.get()
        assert out["status"] == 200
        assert out["headers"]["content-type"].startswith("text/html")
        assert "Hello Fusion!" in out["body"]
