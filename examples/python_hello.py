from fusion_framework import status
from fusion_framework.api import FusionBaseApi
from fusion_framework.http import HTTPException
from fusion_framework.route import router


@router("/api/[module]/{id}")
class MyFirstModule(FusionBaseApi):
    # resolves to /api/myfirst/{id}
    def get(self, id: int):
        return self.response(
            {"message": f"hello from python, id={id}"},
            status=status.HTTP_SUCCESS,
        )

    def post(self, id: int, title: str = "untitled"):
        # id ← path, title ← JSON body (optional via default)
        return self.response(
            {"message": f"created id={id}", "title": title},
            status=status.HTTP_SUCCESS,
        )

    def put(self, id: int):
        return self.response(
            {"message": f"hello from python, id={id}"},
            status=status.HTTP_SUCCESS,
        )

    def delete(self, id: int):
        return self.response(
            {"message": f"hello from python, id={id}"},
            status=status.HTTP_SUCCESS,
        )

    def patch(self, id: int):
        return self.response(
            {"message": f"hello from python, id={id}"},
            status=status.HTTP_SUCCESS,
        )


@router("/api/[module]/")
class ProductModule(FusionBaseApi):
    # resolves to /api/product/
    def get(self, id: int):
        # GET /api/product/?id=12 — missing id arrives as None
        if not id:
            raise HTTPException(400, {"message": "undefined id"})
        return self.response({"products_id": id}, status=status.HTTP_SUCCESS)

    def post(self, id: int):
        # POST /api/product/  body={"id": 12}
        if not id:
            raise HTTPException(400, {"message": "undefined id"})
        return self.response({"products_id": id}, status=status.HTTP_SUCCESS)


@router("/")
class RootApi(FusionBaseApi):
    def get(self):
        return self.response("hello sadfksfiuujisdf0ovjkr9fiek")

    async def post(self):
        return self.response("async post ok")
