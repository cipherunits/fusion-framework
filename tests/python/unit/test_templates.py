"""Tests for Tera template rendering."""

from __future__ import annotations

import asyncio
import inspect
from pathlib import Path

import pytest

from fusion_framework.template import FusionBaseTemplate, render_template


def test_render_builtin_button_macro():
    root = Path(__file__).resolve().parents[2] / "fixtures" / "templates"
    html = render_template("sample/page.html", {}, templates_root=root)
    assert "fusion-btn" in html
    assert "Go" in html


def test_fusion_base_template_context(tmp_path: Path):
    class Page(FusionBaseTemplate):
        template = "hello.html"

        def context(self):
            return {"name": "Fusion"}

    (tmp_path / "hello.html").write_text("<p>Hello {{ name }}!</p>", encoding="utf-8")

    page = Page({"method": "GET", "path": "/"})
    page.templates_dir = str(tmp_path)
    out = page.render()
    assert out["status"] == 200
    assert out["headers"]["content-type"].startswith("text/html")
    assert "Hello Fusion!" in out["body"]


def test_template_name_required():
    class Bad(FusionBaseTemplate):
        pass

    with pytest.raises(ValueError, match="template"):
        Bad({}).template_name()


def test_template_get_returns_json_with_accept(tmp_path: Path):
    class Page(FusionBaseTemplate):
        template = "hello.html"

        def context(self):
            return {"name": "Fusion"}

    (tmp_path / "hello.html").write_text("<p>Hello {{ name }}!</p>", encoding="utf-8")

    page = Page(
        {
            "method": "GET",
            "path": "/pages/home",
            "headers": {"accept": "application/json"},
        }
    )
    page.templates_dir = str(tmp_path)
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


def test_async_context_renders_html(tmp_path: Path):
    """async def context() is awaited by get()/render()."""

    class Page(FusionBaseTemplate):
        template = "hello.html"

        async def context(self):
            return {"name": "AsyncFusion"}

    (tmp_path / "hello.html").write_text("<p>Hello {{ name }}!</p>", encoding="utf-8")
    page = Page({"method": "GET", "path": "/"})
    page.templates_dir = str(tmp_path)

    out = page.get()
    assert inspect.isawaitable(out)
    resolved = asyncio.run(out)
    assert resolved["status"] == 200
    assert "Hello AsyncFusion!" in resolved["body"]


def test_async_context_json_accept():
    class Page(FusionBaseTemplate):
        template = "hello.html"

        async def context(self):
            return {"title": "from-db"}

    page = Page(
        {
            "method": "GET",
            "path": "/",
            "headers": {"accept": "application/json"},
        }
    )
    out = page.get()
    assert inspect.isawaitable(out)
    assert asyncio.run(out) == {"title": "from-db"}


def test_template_get_returns_html_for_browser_accept(tmp_path: Path):
    class Page(FusionBaseTemplate):
        template = "hello.html"

        def context(self):
            return {"name": "Fusion"}

    (tmp_path / "hello.html").write_text("<p>Hello {{ name }}!</p>", encoding="utf-8")

    page = Page(
        {
            "method": "GET",
            "path": "/pages/home",
            "headers": {
                "accept": "text/html,application/xhtml+xml,application/xml;q=0.9"
            },
        }
    )
    page.templates_dir = str(tmp_path)
    out = page.get()
    assert out["status"] == 200
    assert out["headers"]["content-type"].startswith("text/html")
    assert "Hello Fusion!" in out["body"]
