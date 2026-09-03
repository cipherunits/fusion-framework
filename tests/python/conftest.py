"""Shared pytest fixtures for Fusion Framework Python tests."""

from __future__ import annotations

import pytest

pytest.importorskip("fusion_framework")

from fusion_framework._fusion import clear_routes
from fusion_framework.middleware import clear_active_global


@pytest.fixture(autouse=True)
def _isolate_fusion_state():
    """Reset route registry and global middleware between tests."""
    clear_routes()
    clear_active_global()
    yield
    clear_routes()
    clear_active_global()
