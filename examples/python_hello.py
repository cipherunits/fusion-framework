from fusion_framework import status
from fusion_framework.api import FusionBaseApi
from fusion_framework.route import router


@router("/api/[module]/{id}")
class MyFirstModule(FusionBaseApi):
    # resolves to /api/myfirst/{id}
    def get(self, id: int):
        return self.response(
            {"message": f"hello from python, id={id}"},
            status=status.HTTP_SUCCESS,
        )

    def post(self, id: int):
        return self.response(
            {"message": f"hello from python, id={id}"},
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


@router("/")
class RootApi(FusionBaseApi):
    def get(self):
        return self.response("hello sadfksfiuujisdf0ovjkr9fiek")

    async def post(self):
        return self.response("async post ok")
