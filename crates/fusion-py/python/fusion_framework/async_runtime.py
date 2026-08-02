"""Shared asyncio loop for real concurrent async handlers."""

from __future__ import annotations

import asyncio
import threading
from collections.abc import Coroutine
from concurrent.futures import Future
from typing import Any

_lock = threading.Lock()
_loop: asyncio.AbstractEventLoop | None = None
_thread: threading.Thread | None = None


def get_loop() -> asyncio.AbstractEventLoop:
    global _loop, _thread
    with _lock:
        if _loop is not None:
            return _loop
        loop = asyncio.new_event_loop()

        def _run() -> None:
            asyncio.set_event_loop(loop)
            loop.run_forever()

        thread = threading.Thread(target=_run, name="fusion-asyncio", daemon=True)
        thread.start()
        _loop = loop
        _thread = thread
        return loop


def submit(coro: Coroutine[Any, Any, Any]) -> Future[Any]:
    """Schedule ``coro`` on the shared loop; returns a concurrent Future."""
    return asyncio.run_coroutine_threadsafe(coro, get_loop())
