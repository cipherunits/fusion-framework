"""Tokio background tasks — usage example.

Process-wide fire-and-forget / delayed jobs (not a durable queue).

    python examples/background_tasks.py

API:

    from fusion_framework import tasks

    tid = tasks.spawn(fn)                 # run ASAP on Tokio
    tid = tasks.spawn_after(ms, fn)       # delay then run
    tasks.cancel(tid)                     # abort pending/running
    tasks.status(tid)                     # pending|running|done|cancelled|failed
"""

from __future__ import annotations

import time

from fusion_framework import tasks


def main() -> None:
    tasks.reset()
    done = {"n": 0}

    def work() -> None:
        done["n"] += 1
        print("  work() ran")

    # 1) Fire-and-forget
    tid = tasks.spawn(work)
    print(f"spawn     id={tid}  status={tasks.status(tid)}")
    for _ in range(50):
        if done["n"] >= 1 and tasks.status(tid) == "done":
            break
        time.sleep(0.02)
    print(f"          status={tasks.status(tid)}  count={done['n']}")

    # 2) Delayed start, then let it finish
    delayed = tasks.spawn_after(150, work)
    print(f"after     id={delayed}  status={tasks.status(delayed)}")
    for _ in range(50):
        if done["n"] >= 2 and tasks.status(delayed) == "done":
            break
        time.sleep(0.02)
    print(f"          status={tasks.status(delayed)}  count={done['n']}")

    # 3) Cancel before the delay elapses
    doomed = tasks.spawn_after(500, work)
    print(f"cancel    id={doomed}  cancelled={tasks.cancel(doomed)}")
    time.sleep(0.2)
    print(f"          status={tasks.status(doomed)}  count={done['n']} (unchanged)")


if __name__ == "__main__":
    main()
