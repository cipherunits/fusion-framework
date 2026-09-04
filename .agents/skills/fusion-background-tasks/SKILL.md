---
name: fusion-background-tasks
description: >-
  Documents Fusion process-wide Tokio background tasks (spawn, spawn_after,
  cancel, status, snapshot) across Python, Node, and C#. Use when scheduling
  fire-and-forget or delayed work off the request path, or when inspecting tasks
  via the Fusion monitor panel.
---

# Fusion background tasks

In-process jobs on a dedicated **Tokio** multi-thread runtime in `fusion-core`.
Not a durable queue (no Redis/persistence).

## API

| Python | Node | C# |
|--------|------|-----|
| `tasks.spawn(fn)` | `tasks.spawn(fn)` | `BackgroundTasks.Spawn(action)` |
| `tasks.spawn_after(ms, fn)` | `tasks.spawnAfter(ms, fn)` | `BackgroundTasks.SpawnAfter(ms, action)` |
| `tasks.cancel(id)` | `tasks.cancel(id)` | `BackgroundTasks.Cancel(id)` |
| `tasks.status(id)` | `tasks.status(id)` | `BackgroundTasks.Status(id)` |
| `tasks.snapshot()` | `tasks.snapshot()` | `BackgroundTasks.Snapshot()` |
| `tasks.reset()` | `tasks.reset()` | `BackgroundTasks.Reset()` |

Status values: `pending` | `running` | `done` | `cancelled` | `failed`.

`snapshot()` returns `{ task_count, active_count, tasks: [{ id, status, delay_ms, created_at_ms }] }`.
Terminal tasks are pruned (keep last 100) so the registry cannot grow forever.

### Pass a callable

```python
# Correct — defer the call:
tasks.spawn(lambda: test_task(name))

# Wrong — calls test_task immediately and passes None:
# tasks.spawn(test_task(name))  # TypeError: callback must be callable
```

## Fusion monitor

When `monitor.enabled` is true, the Fusion monitor HTML and `{path}/json` embed the
task list under `tasks` / a **Background tasks** card. Settings live under top-level
`monitor.*` (not under `cache.monitor`).

## Notes

- Callbacks may run on Tokio worker threads (Python holds the GIL only for the call).
- Tasks are process-wide and outlive HTTP requests.
- Cancel before run aborts the delay; host userdata is freed (C# GCHandle).

## Examples

`examples/background_tasks.py` / `.mjs` / `.cs`  
`examples/monitor.*` (spawns sample tasks for the panel)

## Implementation

- Core: `crates/fusion-core/src/tasks.rs`
- Python: `fusion_framework.tasks`
- Node: `tasks` export
- C#: `BackgroundTasks` + FFI
