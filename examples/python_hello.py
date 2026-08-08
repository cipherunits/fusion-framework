from fusion_framework.api import FusionBaseApi
from fusion_framework.route import router


@router("/api/[module]/{id}")
class MyFirstModule(FusionBaseApi):
    # resolves to /api/myfirst/{id}
    def get(self, id: int):
        return {"status": 200, "body": f"hello from python, id={id}"}

    def post(self, id: int):
        return {"status": 200, "body": f"hello from python, id={id}"}

    def put(self, id: int):
        return {"status": 200, "body": f"hello from python, id={id}"}

    def delete(self, id: int):
        return {"status": 200, "body": f"hello from python, id={id}"}

    def patch(self, id: int):
        return {"status": 200, "body": f"hello from python, id={id}"}


@router("/")
class RootApi(FusionBaseApi):
    def get(self):
        return self.ok("hello sadfksfiuujisdf0ovjkr9fiek")

    async def post(self):
        return self.ok("async post ok")
