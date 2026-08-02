from __future__ import annotations

from typing import Any, Type

from fusion_framework._fusion import App
from fusion_framework.api import FusionBaseApi
from fusion_framework.route import invoke_api_method, registered_routes
from fusion_framework.config import Settings, get_settings, load_settings_module, settings as settings_store


class FusionApp:
    """Class-based application facade over the Rust ``App`` engine."""

    def __init__(self, app_settings: Settings | None = None):
        self.settings = app_settings or get_settings()
        self._engine = App()
        self._mounted = False

    def mount(self) -> None:
        if self._mounted:
            return
        for path, api_cls in registered_routes():
            self._mount_api(path, api_cls)
        self._mounted = True

    def _mount_api(self, path: str, api_cls: Type[FusionBaseApi]) -> None:
        for method_name in FusionBaseApi.HTTP_METHODS:
            if not _defines_method(api_cls, method_name):
                continue

            def make_handler(cls: Type[FusionBaseApi], http_method: str):
                def handler(request: dict[str, Any]):
                    return invoke_api_method(cls, http_method, request)

                return handler

            self._engine.route(method_name.upper(), path, make_handler(api_cls, method_name))

    def listen(self, host: str | None = None, port: int | None = None) -> None:
        self.mount()
        host = host if host is not None else self.settings.host
        port = port if port is not None else self.settings.port
        if self.settings.debug:
            print(f"fusion listening on http://{host}:{port}")
        self._engine.listen(host, int(port))


def _defines_method(api_cls: Type[FusionBaseApi], method_name: str) -> bool:
    for cls in api_cls.__mro__:
        if cls is FusionBaseApi or cls is object:
            break
        if method_name in cls.__dict__:
            return True
    return False


def run(settings_module: str | None = "settings") -> None:
    """Start the server using JSON + optional Python settings module.

    API classes must already be imported (so ``@router`` has registered them).
    """
    if settings_module:
        load_settings_module(settings_module)
    else:
        settings_store.load_json()
    FusionApp(get_settings()).listen()
