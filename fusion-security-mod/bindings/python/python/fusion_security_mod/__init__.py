"""Fusion module `security` (Rust core via PyO3)."""

from . import _native


def hello(name: str = "Fusion") -> str:
    return _native.hello(name)


__all__ = ["hello"]
