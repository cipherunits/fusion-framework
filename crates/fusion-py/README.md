# fusion-framework (Python)

نسخه‌ی پایتونِ کلاس‌محور برای `fusion-core` (هسته‌ی Rust).

در این طراحی، منطق‌های اصلی مثل routing، parsing، serialization و **binding پارامترهای handler** در Rust انجام می‌شود. سمت پایتون فقط interface و DX باقی می‌ماند.

## نصب

```bash
pip install fusion-framework
```

## شروع سریع

```python
from fusion_framework.api import FusionBaseApi
from fusion_framework.route import router
from fusion_framework.app import run

 
@router("/api/[module]/{id}")
class MyModule(FusionBaseApi):
    def get(self, id: int):
        # `id` طبق annotation به int تبدیل می‌شود (در Rust)
        return self.response({"ok": True, "id": id}, status=200)


if __name__ == "__main__":
    run()
```

## تنظیمات Swagger

در `fusion.<env>.json` (همان فایلی که `fusion init` می‌سازد) بلاک `swagger` را ویرایش کن:

```json
{
  "env": "dev",
  "config": {
    "host": "127.0.0.1",
    "port": 8080,
    "swagger": {
      "enabled": true,
      "path": "/swagger",
      "title": "Fusion API Docs",
      "info": {
        "title": "Fusion API",
        "version": "1.0.0",
        "description": "My API docs"
      },
      "ui": {
        "deepLinking": true,
        "docExpansion": "list",
        "filter": true,
        "tryItOutEnabled": true,
        "persistAuthorization": false,
        "displayRequestDuration": true
      }
    }
  }
}
```

- `enabled` / `path`: روشن/خاموش و آدرس UI (پیش‌فرض `/swagger`)
- `info`: فیلدهای OpenAPI `info` (title/version/description/...)
- `ui`: گزینه‌های [Swagger UI](https://swagger.io/docs/open-source-tools/swagger-ui/usage/configuration/)

متادیتای هر endpoint همچنان از `@route(..., tags=..., desc=..., title=..., deprecated=...)` می‌آید.

## قرارداد Handlerها

متدهای HTTP مثل `get/post/put/delete/patch` را به صورت معمول پیاده کن.

1. ورودی‌ها از طریق امضای متد (`def get(self, id: int, ...)`) گرفته می‌شوند.
2. منبع مقدار بر اساس نام پارامتر:
  - اگر نام با `path params` یکی باشد → از path
  - اگر متد `POST/PUT/PATCH` باشد و نام در body JSON باشد → از body
  - در غیر این صورت اگر نام در `query` باشد → از query
3. اگر پارامتری annotation/optional داشته باشد و وجود نداشته باشد → `None` ارسال می‌شود.



## خطاها

```python
from fusion_framework.http import HTTPException

raise HTTPException(404, {"detail": "not found"})
```



## فایل‌های مهم

- `fusion_framework.api.FusionBaseApi`
  - view درخواست: `method/path/body/headers/params/query`
  - helper envelope: `response(...)`
- `fusion_framework.route.router`
  - ثبت کلاس‌ها با template مثل `/api/[module]/{id}`
- `fusion_framework.app.run`
  - settings را لود می‌کند و برنامه را mount/listen می‌کند
- پشت صحنه:
  - `crates/fusion-py/src/lib.rs`: PyO3 bridge
  - `crates/fusion-core/src/*`: routing/serialize/binding مشترک

