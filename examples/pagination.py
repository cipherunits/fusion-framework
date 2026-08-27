"""Paginated list API example."""

from fusion_framework import status
from fusion_framework.api import FusionBaseApi
from fusion_framework.app import FusionApp
from fusion_framework.config import get_settings, load_settings_module
from fusion_framework.route import route

ALL_PRODUCTS = [{"id": i, "name": f"product-{i}"} for i in range(1, 51)]


@route("/api/[module]", tags=["products"], version="v1")
class ProductModule(FusionBaseApi):
    def get(self, page: int = 1, page_size: int = 20):
        # GET /v1/api/product/?page=2&page_size=10
        params = self.pagination(page=page, page_size=page_size)
        start = int(params.offset)
        end = start + int(params.limit)
        items = ALL_PRODUCTS[start:end]
        return self.paginated(items, total=len(ALL_PRODUCTS), params=params, status=status.HTTP_SUCCESS)


def main() -> None:
    load_settings_module("settings")
    FusionApp(get_settings()).listen()


if __name__ == "__main__":
    main()
