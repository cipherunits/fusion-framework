"""Unit tests for the process-wide Fusion cache (moka)."""

from __future__ import annotations

import asyncio

from fusion_framework import cache


def setup_function() -> None:
    cache.reset()
    cache.configure_driver("moka", default_ttl=None)


def test_set_get_delete_exists():
    assert cache.get("k") is None
    cache.set("k", {"n": 1})
    assert cache.exists("k")
    assert cache.get("k") == {"n": 1}
    assert cache.delete("k") is True
    assert cache.exists("k") is False


def test_get_or_set_callable():
    calls = {"n": 0}

    def factory():
        calls["n"] += 1
        return {"v": calls["n"]}

    assert cache.get_or_set("x", factory) == {"v": 1}
    assert cache.get_or_set("x", factory) == {"v": 1}
    assert calls["n"] == 1


def test_exists_or_set_and_delete_or_set():
    assert cache.exists_or_set("f", True) is False
    assert cache.exists_or_set("f", False) is True
    assert cache.get("f") is True
    assert cache.delete_or_set("f", "next") == "next"
    assert cache.get("f") == "next"


def test_clear_removes_all():
    cache.set("a", 1)
    cache.set("b", 2)
    cache.clear()
    assert cache.get("a") is None
    assert cache.get("b") is None


def test_explicit_ttl_expires():
    cache.set("short", "x", ttl=0.05)
    assert cache.exists("short")
    import time

    time.sleep(0.08)
    assert cache.get("short") is None


def test_omitted_ttl_stays_forever_with_null_default():
    cache.set("forever", "x")
    import time

    time.sleep(0.05)
    assert cache.get("forever") == "x"


def test_driver_is_moka():
    assert cache.driver() == "moka"


def test_mako_alias():
    cache.reset()
    cache.configure_driver("mako", default_ttl=None)
    assert cache.driver() == "moka"


def test_async_set_get_clear():
    async def body() -> None:
        await cache.aset("async-k", {"ok": True})
        assert await cache.aget("async-k") == {"ok": True}
        assert await cache.aexists("async-k") is True
        await cache.aclear()
        assert await cache.aget("async-k") is None

    asyncio.run(body())


def test_async_get_or_set_async_factory():
    async def body() -> None:
        calls = {"n": 0}

        async def factory():
            await asyncio.sleep(0)
            calls["n"] += 1
            return {"v": calls["n"]}

        assert await cache.aget_or_set("ax", factory) == {"v": 1}
        assert await cache.aget_or_set("ax", factory) == {"v": 1}
        assert calls["n"] == 1
        assert await cache.aexists_or_set("flag", True) is False
        assert await cache.aexists_or_set("flag", False) is True
        assert await cache.adelete_or_set("flag", "next") == "next"
        assert await cache.adelete("flag") is True

    asyncio.run(body())
