# fusion-framework (Node.js)

یک لایه‌ی API-کلاس (class-based) برای Node.js که روی هسته‌ی مشترک `fusion-core` کار می‌کند.
در این نسخه، منطق بایندینگ/پارِسینگ درخواست (path/query/body) و ساخت پاسخ‌ها از `index.js` حذف شده و در Rust (`fusion-core`) انجام می‌شود. بنابراین سمت Node کافی است متدهای API را بدون آرگومان بنویسی و از `this.params / this.query / this.body` استفاده کنی.

## نصب

در پروژه‌ات داخل پوشه‌ی `crates/fusion-node`:

```bash
npm install
npm run build:debug
```

یا اگر پکیج منتشر شده باشد:

```bash
npm i fusion-framework
```

## شروع سریع

فایل `examples/node_hello.mjs` را مشابه زیر بنویس:

```js
import { FusionBaseApi, router, FusionApp } from 'fusion-framework'

export const MyModule = router('/api/[module]/{id}')(
  class MyModule extends FusionBaseApi {
    get() {
      return { status: 200, body: `hello id=${this.params.id}` }
    }
  }
)

new FusionApp().listen()
```

## قرارداد متدهای API (مهم)

- متدهای HTTP مثل `get() / post() / put() / delete() / patch()` باید **بدون آرگومان** باشند.
- مقدارها را از:
  - `this.params` (path params به صورت string)
  - `this.query` (query params به صورت string)
  - `this.body` (body به صورت string)
  - `this.headers`
  - `this.method`, `this.path`
  بردار.
- برای خطا:
  - `throw new HTTPException(status, detail, headers?)`

## `HTTPException`

یک helper نازک برای ساخت response envelope است. در سمت `fusion-core`:
- اگر `body` آبجکت/JSON باشد، `content-type` به صورت خودکار ست می‌شود.

## فایل‌های مهم

- `crates/fusion-node/index.js`
  - DX کلاس‌محور: `FusionBaseApi`، `router()` برای ثبت routeها، `FusionApp` برای mount/listen
  - هیچ منطق بایندینگ درخواست ندارد (فقط invoke ساده‌ی متد بدون آرگومان)
- `crates/fusion-node/src/lib.rs`
  - Native addon (N-API) که درخواست HTTP را از `fusion-core` می‌گیرد
  - یک object مطابق `FusionRequest` به JS می‌دهد
  - return value را تبدیل به JSON می‌کند و به `fusion-core` برمی‌گرداند تا response واقعی ساخته شود
- `crates/fusion-node/index.d.ts`
  - TypeScript typings برای API.

## تنظیمات Swagger

در `fusion.<env>.json`:

```json
{
  "config": {
    "swagger": {
      "enabled": true,
      "path": "/swagger",
      "title": "Fusion API Docs",
      "info": { "title": "Fusion API", "version": "1.0.0", "description": "..." },
      "servers": [{ "url": "/", "description": "Current host" }],
      "auth": {
        "persistAuthorization": true,
        "schemes": {
          "BearerAuth": { "type": "http", "scheme": "bearer", "bearerFormat": "JWT" }
        },
        "global": [],
        "oauth": { "clientId": "", "appName": "Fusion API", "usePkceWithAuthorizationCodeGrant": true }
      },
      "navbar": { "enabled": true, "showUrlInput": true },
      "ui": { "docExpansion": "list", "filter": true, "tryItOutEnabled": true }
    }
  }
}
```

- `auth.schemes` / `auth.global` → OpenAPI security
- `auth.oauth` → `ui.initOAuth(...)`
- `navbar.enabled` → Topbar / StandaloneLayout
- `ui.*` → گزینه‌های Swagger UI Bundle

