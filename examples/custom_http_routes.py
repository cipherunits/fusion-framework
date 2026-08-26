"""Custom HTTP method routes — convention get/post plus @http_get handlers."""

from fusion_framework import status
from fusion_framework.api import FusionBaseApi
from fusion_framework.app import FusionApp
from fusion_framework.config import get_settings, load_settings_module
from fusion_framework.http_route import http_get
from fusion_framework.route import route


@route("/api/[module]", tags=["users"])
class UserModule(FusionBaseApi):
    # Convention route: GET /api/user
    def get(self):
        return self.response({"mode": "convention"}, status=status.HTTP_SUCCESS)

    # Custom route: GET /api/user/test/user  ([action] -> user)
    @http_get("test/[action]", title="User action")
    def UserAction(self):
        return self.response({"mode": "custom", "action": "user"}, status=status.HTTP_SUCCESS)


def main() -> None:
    load_settings_module("settings")
    FusionApp(get_settings()).listen()


if __name__ == "__main__":
    main()
