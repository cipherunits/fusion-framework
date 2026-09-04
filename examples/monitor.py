"""Built-in Fusion monitor demo (cache + background tasks).

Enable via fusion.<env>.json:

    "monitor": { "enabled": true, "path": "/__fusion/monitor" },
    "cache": { "driver": "moka", "max_events": 50 }

When monitor.enabled is false, FusionApp does not register HTML or /json endpoints.

    python examples/monitor.py
"""

from __future__ import annotations

from fusion_framework import cache, tasks
from fusion_framework._fusion import Settings


def main() -> None:
    settings = Settings()
    settings.merge(
        {
            "monitor": {
                "enabled": True,
                "path": "/__fusion/monitor",
            },
            "cache": {
                "driver": "moka",
                "default_ttl": None,
                "max_events": 50,
            },
        }
    )
    cache.configure(settings)
    cache.set("demo:user", {"name": "Ada"}, ttl=60)

    tasks.reset()
    # Pass a callable — not the result of calling the function:
    tasks.spawn(lambda: cache.set("from:task", True))
    tasks.spawn_after(5_000, lambda: None)

    snap = cache.snapshot()
    task_info = snap["tasks"]
    print(
        f"driver={snap['driver']} keys={snap['entry_count']} events={snap['event_count']}"
    )
    print(
        f"tasks active={task_info['active_count']} total={task_info['task_count']}"
    )
    print(f"open http://127.0.0.1:8080{snap['monitor']['path']} after listen()")


if __name__ == "__main__":
    main()
