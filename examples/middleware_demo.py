"""Middleware demo: global JWT decode + route permission guards.

Run::

    python examples/middleware_demo.py

Try::

    curl http://127.0.0.1:3010/api/admin
    # -> 401 (no token)

    # payload: {"sub":"1","roles":["admin"]}  (unsigned demo JWT body)
    TOKEN="eyJhbGciOiJub25lIiwidHlwIjoiSldUIn0.eyJzdWIiOiIxIiwicm9sZXMiOlsiYWRtaW4iXX0."

    curl -H "Authorization: Bearer $TOKEN" http://127.0.0.1:3010/api/admin
    # -> 200

    curl -H "Authorization: Bearer $TOKEN" http://127.0.0.1:3010/api/super
    # -> 403 (needs super_admin)
"""

from fusion_framework import bearer_jwt, status
from fusion_framework.api import FusionBaseApi
from fusion_framework.app import FusionApp
from fusion_framework.config import get_settings, load_settings_module
from fusion_framework.route import route


def is_admin(request):
    jwt = (request.get("state") or {}).get("jwt") or {}
    roles = jwt.get("roles") or []
    if isinstance(roles, str):
        roles = [roles]
    return "admin" in roles or "super_admin" in roles


def is_super_admin(request):
    jwt = (request.get("state") or {}).get("jwt") or {}
    roles = jwt.get("roles") or []
    if isinstance(roles, str):
        roles = [roles]
    return "super_admin" in roles


@route("/api/admin", permissions=[is_admin])
class AdminModule(FusionBaseApi):
    def get(self):
        user = self.state.get("jwt", {})
        return self.response(
            {"message": "admin area", "user": user.get("sub")},
            status=status.HTTP_SUCCESS,
        )


@route("/api/super", permissions=[is_super_admin])
class SuperModule(FusionBaseApi):
    def get(self):
        return self.response({"message": "super admin only"}, status=status.HTTP_SUCCESS)


def main() -> None:
    load_settings_module("settings")
    app = FusionApp(get_settings())
    app.use(bearer_jwt())
    app.listen()


if __name__ == "__main__":
    main()
