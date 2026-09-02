"""Process-based auto-reload for development (stdlib only).

Parent process watches source files; on change it restarts a child that runs
the real server. Disable with ``listen(reload=False)`` or ``reload: false``.
"""

from __future__ import annotations

import os
import signal
import subprocess
import sys
import time
from pathlib import Path
from typing import Iterable, Sequence

ENV_CHILD = "FUSION_RELOAD_CHILD"

# Default: watch common project source extensions.
DEFAULT_EXTENSIONS = (
    ".py",
    ".html",
    ".tera",
    ".json",
    ".js",
    ".mjs",
    ".cjs",
    ".ts",
    ".cs",
)

SKIP_DIR_NAMES = {
    ".git",
    ".hg",
    ".svn",
    ".venv",
    "venv",
    "node_modules",
    "target",
    "__pycache__",
    ".mypy_cache",
    ".pytest_cache",
    ".ruff_cache",
    "bin",
    "obj",
    "dist",
    "build",
    ".idea",
    ".vscode",
}


def is_reload_child() -> bool:
    return os.environ.get(ENV_CHILD) == "1"


def resolve_reload(
    reload: bool | None,
    *,
    settings_reload: bool,
) -> bool:
    """Explicit ``reload=`` wins; otherwise use settings (default false)."""
    if reload is not None:
        return bool(reload)
    return bool(settings_reload)


def _iter_files(roots: Sequence[Path], extensions: Sequence[str]) -> Iterable[Path]:
    ext_set = {e if e.startswith(".") else f".{e}" for e in extensions}
    for root in roots:
        if not root.exists():
            continue
        if root.is_file():
            if root.suffix.lower() in ext_set:
                yield root
            continue
        for dirpath, dirnames, filenames in os.walk(root):
            dirnames[:] = [d for d in dirnames if d not in SKIP_DIR_NAMES]
            for name in filenames:
                path = Path(dirpath) / name
                if path.suffix.lower() in ext_set:
                    yield path


def snapshot_mtimes(roots: Sequence[Path], extensions: Sequence[str]) -> dict[str, float]:
    out: dict[str, float] = {}
    for path in _iter_files(roots, extensions):
        try:
            out[str(path)] = path.stat().st_mtime
        except OSError:
            continue
    return out


def changed_since(
    previous: dict[str, float],
    roots: Sequence[Path],
    extensions: Sequence[str],
) -> list[str]:
    current = snapshot_mtimes(roots, extensions)
    changed: list[str] = []
    for path, mtime in current.items():
        old = previous.get(path)
        if old is None or mtime > old:
            changed.append(path)
    for path in previous:
        if path not in current:
            changed.append(path)
    return changed


def default_watch_roots() -> list[Path]:
    roots: list[Path] = [Path.cwd()]
    main = sys.modules.get("__main__")
    main_file = getattr(main, "__file__", None)
    if main_file:
        roots.append(Path(main_file).resolve().parent)
    # Unique, existing paths
    seen: set[str] = set()
    unique: list[Path] = []
    for root in roots:
        key = str(root.resolve()) if root.exists() else str(root)
        if key in seen:
            continue
        seen.add(key)
        unique.append(root)
    return unique


def run_with_reloader(
    *,
    watch_dirs: Sequence[str | Path] | None = None,
    extensions: Sequence[str] | None = None,
    poll_interval: float = 0.5,
) -> None:
    """Parent loop: spawn this same process as a child and restart on changes."""
    if is_reload_child():
        raise RuntimeError("run_with_reloader must not run inside the child process")

    roots = [Path(p) for p in watch_dirs] if watch_dirs else default_watch_roots()
    exts = tuple(extensions) if extensions else DEFAULT_EXTENSIONS
    env = os.environ.copy()
    env[ENV_CHILD] = "1"

    print(
        f"fusion: reload enabled (watching {', '.join(str(r) for r in roots)})",
        flush=True,
    )

    process: subprocess.Popen[bytes] | None = None

    def stop_child() -> None:
        nonlocal process
        if process is None or process.poll() is not None:
            process = None
            return
        process.send_signal(signal.SIGTERM)
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=2)
        process = None

    def start_child() -> subprocess.Popen[bytes]:
        return subprocess.Popen([sys.executable, *sys.argv], env=env)

    def handle_signal(signum: int, _frame) -> None:
        stop_child()
        raise SystemExit(128 + signum)

    signal.signal(signal.SIGINT, handle_signal)
    signal.signal(signal.SIGTERM, handle_signal)

    mtimes = snapshot_mtimes(roots, exts)
    process = start_child()

    try:
        while True:
            time.sleep(poll_interval)
            if process.poll() is not None:
                # Child exited on its own — restart after a short pause.
                code = process.returncode
                print(f"fusion: child exited ({code}); restarting…", flush=True)
                time.sleep(0.3)
                process = start_child()
                mtimes = snapshot_mtimes(roots, exts)
                continue

            dirty = changed_since(mtimes, roots, exts)
            if not dirty:
                continue
            rel = dirty[0]
            try:
                rel = str(Path(rel).relative_to(Path.cwd()))
            except ValueError:
                pass
            print(f"fusion: change detected ({rel}); reloading…", flush=True)
            stop_child()
            process = start_child()
            mtimes = snapshot_mtimes(roots, exts)
    finally:
        stop_child()
