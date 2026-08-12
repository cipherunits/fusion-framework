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
    "swagger": {
      "enabled": true,
      "path": "/swagger",
      "title": "Fusion API Docs",
      "info": {
        "title": "Fusion API",
        "version": "1.0.0",
        "description": "My API docs",
        "contact": { "name": "API Support", "email": "support@example.com" },
        "license": { "name": "MIT" }
      },
      "servers": [{ "url": "/", "description": "Current host" }],
      "auth": {
        "persistAuthorization": true,
        "schemes": {
          "BearerAuth": { "type": "http", "scheme": "bearer", "bearerFormat": "JWT" },
          "ApiKeyAuth": { "type": "apiKey", "in": "header", "name": "X-API-Key" }
        },
        "global": [],
        "oauth": {
          "clientId": "",
          "appName": "Fusion API",
          "scopes": "",
          "usePkceWithAuthorizationCodeGrant": true
        }
      },
      "navbar": {
        "enabled": true,
        "showUrlInput": true
      },
      "ui": {
        "deepLinking": true,
        "docExpansion": "list",
        "filter": true,
        "tryItOutEnabled": true,
        "displayRequestDuration": true
      }
    }
  }
}
```

### معنی کلیدها

| کلید | کار |
|------|-----|
| `enabled` | روشن/خاموش کردن کل Swagger |
| `path` | آدرس UI (پیش‌فرض `/swagger`) |
| `title` | عنوان تب مرورگر |
| `info` | متادیتای OpenAPI (`title`/`version`/`description`/`contact`/`license`) |
| `servers` | لیست سرورها در OpenAPI (Try it out روی کدام host بزند) |
| `auth.schemes` | تعریف روش‌های احراز هویت در OpenAPI (Bearer / API Key / OAuth2 / ...) |
| `auth.global` | اگر پر باشد، همه‌ی endpointها به‌صورت پیش‌فرض آن security را می‌گیرند |
| `auth.persistAuthorization` | توکن Authorize را بعد از رفرش صفحه نگه می‌دارد |
| `auth.oauth` | تنظیمات `initOAuth` برای جریان OAuth2 در UI |
| `navbar.enabled` | نوار بالای Swagger (Topbar) را نشان بده/پنهان کن |
| `navbar.showUrlInput` | اینپوت آدرس spec داخل navbar |
| `navbar.urls` | چند spec برای سوئیچ از navbar: `[{ "url": "...", "name": "..." }]` |
| `ui.*` | بقیه‌ی گزینه‌های [Swagger UI](https://swagger.io/docs/open-source-tools/swagger-ui/usage/configuration/) |

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

