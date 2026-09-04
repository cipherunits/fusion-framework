"""Built-in cache monitor demo (snapshot / panel context).

Enable via fusion.<env>.json:

    "cache": {
      "monitor": { "enabled": true, "path": "/__fusion/cache", "max_events": 50 }
    }

When enabled is false, FusionApp does not register HTML or /json endpoints.

    python examples/cache_monitor.py
"""

from __future__ import annotations

from fusion_framework import cache
from fusion_framework._fusion import Settings


def main() -> None:
    settings = Settings()
    settings.merge(
        {
            "cache": {
                "driver": "moka",
                "default_ttl": None,
                "monitor": {
                    "enabled": True,
                    "path": "/__fusion/cache",
                    "max_events": 50,
                },
            }
        }
    )
    cache.configure(settings)
    cache.set("demo:user", {"name": "Ada"}, ttl=60)

    snap = cache.snapshot()
    print(
        f"driver={snap['driver']} keys={snap['entry_count']} events={snap['event_count']}"
    )
    print(f"open http://127.0.0.1:8080{snap['monitor']['path']} after listen()")

    # from fusion_framework.app import FusionApp
    # FusionApp(settings).listen()


if __name__ == "__main__":
    main()
