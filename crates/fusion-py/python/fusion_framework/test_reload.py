"""Tests for reload helpers (no live server)."""

from __future__ import annotations

import time
from pathlib import Path

from fusion_framework.reload import (
    changed_since,
    resolve_reload,
    snapshot_mtimes,
)


def test_resolve_reload_explicit_wins():
    assert resolve_reload(True, settings_reload=False) is True
    assert resolve_reload(False, settings_reload=True) is False
    assert resolve_reload(None, settings_reload=True) is True
    assert resolve_reload(None, settings_reload=False) is False


def test_snapshot_detects_change(tmp_path: Path):
    watched = tmp_path / "app.py"
    watched.write_text("print(1)\n", encoding="utf-8")
    before = snapshot_mtimes([tmp_path], [".py"])
    time.sleep(0.05)
    watched.write_text("print(2)\n", encoding="utf-8")
    dirty = changed_since(before, [tmp_path], [".py"])
    assert any(str(watched) == p or p.endswith("app.py") for p in dirty)
