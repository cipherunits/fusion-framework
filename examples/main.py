"""Entry point: import API classes, then start from settings."""

import python_hello  # noqa: F401  — registers @router classes
from fusion_framework.app import run

if __name__ == "__main__":
    run("settings")
