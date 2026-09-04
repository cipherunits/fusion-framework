"""Unit tests for Tokio background tasks."""

from __future__ import annotations

import time

from fusion_framework import tasks


def setup_function() -> None:
    tasks.reset()


def _wait_done(tid: str, side_effect, *, timeout_s: float = 2.0) -> None:
    """Poll until status is done and side_effect() is truthy."""
    deadline = time.monotonic() + timeout_s
    while time.monotonic() < deadline:
        if side_effect() and tasks.status(tid) == "done":
            return
        time.sleep(0.02)


def test_spawn_runs():
    box = {"n": 0}

    def work():
        box["n"] += 1

    tid = tasks.spawn(work)
    _wait_done(tid, lambda: box["n"] == 1)
    assert box["n"] == 1
    assert tasks.status(tid) == "done"


def test_spawn_after_runs():
    box = {"n": 0}

    def work():
        box["n"] += 1

    tid = tasks.spawn_after(80, work)
    assert tasks.status(tid) in ("pending", "running")
    time.sleep(0.03)
    assert box["n"] == 0
    _wait_done(tid, lambda: box["n"] == 1)
    assert box["n"] == 1
    assert tasks.status(tid) == "done"


def test_spawn_after_and_cancel():
    box = {"n": 0}

    def work():
        box["n"] += 1

    tid = tasks.spawn_after(400, work)
    assert tasks.status(tid) in ("pending", "running")
    assert tasks.cancel(tid) is True
    time.sleep(0.15)
    assert box["n"] == 0
    assert tasks.status(tid) == "cancelled"


def test_status_unknown_id():
    assert tasks.status("task-does-not-exist") is None
    assert tasks.cancel("task-does-not-exist") is False


def test_snapshot_lists_tasks():
    tid = tasks.spawn_after(5_000, lambda: None)
    snap = tasks.snapshot()
    assert snap["task_count"] >= 1
    assert snap["active_count"] >= 1
    ids = [t["id"] for t in snap["tasks"]]
    assert tid in ids
    assert tasks.cancel(tid) is True
    assert tasks.snapshot()["tasks"][0]["status"] == "cancelled"
