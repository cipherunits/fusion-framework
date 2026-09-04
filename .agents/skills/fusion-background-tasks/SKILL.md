---
name: fusion-background-tasks
description: >-
  Documents Fusion process-wide Tokio background tasks (spawn, spawn_after,
  cancel, status) across Python, Node, and C#. Use when scheduling fire-and-forget
  or delayed work off the request path.
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
| `tasks.reset()` | `tasks.reset()` | `BackgroundTasks.Reset()` |

Status values: `pending` | `running` | `done` | `cancelled` | `failed`.

## Notes

- Callbacks may run on Tokio worker threads (Python holds the GIL only for the call).
- Tasks are process-wide and outlive HTTP requests.
- Cancel before run aborts the delay; host userdata is freed (C# GCHandle).

## Examples

`examples/background_tasks.py` / `.mjs` / `.cs`

## Implementation

- Core: `crates/fusion-core/src/tasks.rs`
- Python: `fusion_framework.tasks`
- Node: `tasks` export
- C#: `BackgroundTasks` + FFI
