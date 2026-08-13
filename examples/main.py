"""Entry point: register routes, middleware, and start the server."""

import python_hello  # noqa: F401  — registers @router classes

from fusion_framework.app import FusionApp
from fusion_framework.config import get_settings, load_settings_module

# Global middleware (optional). Framework ships with none by default.
MIDDLEWARE: list = []


def main() -> None:
    load_settings_module("settings")
    app = FusionApp(get_settings())
    for middleware in MIDDLEWARE:
        app.use(middleware)
    app.listen()


if __name__ == "__main__":
    main()
