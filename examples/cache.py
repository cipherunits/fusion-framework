"""Application cache demo (sync + async, default driver: moka).

    python examples/cache.py
"""

from __future__ import annotations

import asyncio

from fusion_framework import cache


async def async_demo() -> None:
    await cache.aset("async-greeting", {"hello": "async"}, ttl=30)
    print("aget:", await cache.aget("async-greeting"))
    print(
        "aget_or_set:",
        await cache.aget_or_set("async-counter", _async_one),
    )
    await cache.aclear()
    print("after aclear:", await cache.aget("async-greeting"))


async def _async_one() -> int:
    await asyncio.sleep(0)
    return 1


def main() -> None:
    cache.configure_driver("moka", default_ttl=60)
    cache.set("greeting", {"hello": "world"}, ttl=30)
    print("get:", cache.get("greeting"))
    print("exists:", cache.exists("greeting"))
    print("get_or_set:", cache.get_or_set("counter", lambda: 1))
    print("exists_or_set (first):", cache.exists_or_set("flag", True))
    print("exists_or_set (again):", cache.exists_or_set("flag", False))
    print("delete_or_set:", cache.delete_or_set("greeting", {"hello": "fusion"}))
    print("driver:", cache.driver())
    cache.clear()
    print("after clear:", cache.get("greeting"))
    asyncio.run(async_demo())


if __name__ == "__main__":
    main()
