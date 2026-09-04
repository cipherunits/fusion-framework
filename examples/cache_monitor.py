"""Deprecated alias — see examples/monitor.py (Fusion monitor)."""

from __future__ import annotations

# Re-run the same demo as examples/monitor.py without package imports.
import runpy
from pathlib import Path

if __name__ == "__main__":
    runpy.run_path(str(Path(__file__).with_name("monitor.py")), run_name="__main__")
