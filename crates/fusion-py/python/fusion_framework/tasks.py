"""Process-wide Tokio background tasks.

Spawn host callables on Fusion's dedicated Tokio runtime::

    from fusion_framework import tasks

    def work():
        print("done")

    tid = tasks.spawn(work)
    tasks.spawn_after(1000, work)  # delay in milliseconds
    tasks.status(tid)              # pending|running|done|cancelled|failed
    tasks.cancel(tid)
    tasks.snapshot()               # also under cache.snapshot()["tasks"]
"""

from __future__ import annotations

from typing import Any, Callable, Optional

from fusion_framework._fusion import (
    task_cancel as _task_cancel,
    task_reset as _task_reset,
    task_snapshot as _task_snapshot,
    task_spawn as _task_spawn,
    task_spawn_after as _task_spawn_after,
    task_status as _task_status,
)


def spawn(callback: Callable[[], Any]) -> str:
    """Run ``callback`` on the Tokio background runtime. Returns task id.

    Pass a callable, not the result of calling it::

        tasks.spawn(lambda: test_task(name))   # correct
        # tasks.spawn(test_task(name))         # wrong — TypeError
    """
    if not callable(callback):
        raise TypeError(
            "callback must be callable; use tasks.spawn(lambda: work(arg)) "
            "not tasks.spawn(work(arg))"
        )
    return str(_task_spawn(callback))


def spawn_after(delay_ms: int, callback: Callable[[], Any]) -> str:
    """Run ``callback`` after ``delay_ms`` milliseconds. Returns task id."""
    if not callable(callback):
        raise TypeError(
            "callback must be callable; use tasks.spawn_after(ms, lambda: work(arg)) "
            "not tasks.spawn_after(ms, work(arg))"
        )
    if delay_ms < 0:
        raise ValueError("delay_ms must be >= 0")
    return str(_task_spawn_after(int(delay_ms), callback))


def cancel(task_id: str) -> bool:
    """Cancel a pending/running task. Returns whether the id was known."""
    return bool(_task_cancel(str(task_id)))


def status(task_id: str) -> Optional[str]:
    """Return status string, or ``None`` if the id is unknown."""
    value = _task_status(str(task_id))
    return str(value) if value is not None else None


def snapshot() -> dict[str, Any]:
    """JSON snapshot of tracked tasks (also embedded in ``cache.snapshot()['tasks']``)."""
    return dict(_task_snapshot())


def reset() -> None:
    """Abort and clear all tracked tasks (tests)."""
    _task_reset()


__all__ = ["spawn", "spawn_after", "cancel", "status", "snapshot", "reset"]
