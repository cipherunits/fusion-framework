# Fusion Framework (Python)

فریم‌ورک HTTP کلاس‌محور با هستهٔ Rust — سریع، یکپارچه، و مشترک بین Python / Node / C#.

مسیریابی، بایندینگ پارامتر، و سریالایز پاسخ در `fusion-core` اجرا می‌شود؛ پایتون لایهٔ نازک DX است.

## لینک‌ها

- **مستندات:** [fusion.cipherunit.xyz](https://fusion.cipherunit.xyz/)
- **GitHub:** [cipherunits/fusion-framework](https://github.com/cipherunits/fusion-framework)
- **CLI (Fusion Tool):** [cipherunits/fusion-tool](https://github.com/cipherunits/fusion-tool)
- **Desktop:** [fusion.cipherunit.xyz/en/gui](https://fusion.cipherunit.xyz/en/gui)

## نصب

```bash
pip install fusion-framework
```

پروژهٔ جدید با CLI:

```bash
fusion init --lang python --name my-app
cd my-app
pip install fusion-framework
```

## مثال کوتاه

```python
from fusion_framework.api import FusionBaseApi
from fusion_framework.route import route
from fusion_framework import status


@route("api/[module]/{id}", tags=["items"])
class ItemModule(FusionBaseApi):
    def get(self, id: int):
        return self.response({"id": id}, status=status.HTTP_SUCCESS)

    async def post(self, id: int, title: str = "untitled"):
        return self.response(
            {"id": id, "title": title},
            status=status.HTTP_201_CREATED,
        )
```

در `main.py` ماژول را import کنید، تنظیمات را لود کنید، و سرور را بالا بیاورید:

```python
import src.modules.items.items  # noqa: F401

from fusion_framework.app import FusionApp
from fusion_framework.config import get_settings, load_settings_module

MIDDLEWARE: list = []  # middleware اختیاری — فریم‌ورک پیش‌فرض ندارد


def main() -> None:
    load_settings_module("settings")
    app = FusionApp(get_settings())
    for mw in MIDDLEWARE:
        app.use(mw)
    app.listen()


if __name__ == "__main__":
    main()
```

## امکانات

- API کلاس‌محور: `get` / `post` / `put` / `patch` / `delete`
- بایندینگ typed از path / body / query
- Swagger / OpenAPI خودکار
- Middleware سراسری و روی route (بدون پیش‌فرض)
- Handler و middleware همگام و ناهمگام
- تنظیمات از `fusion.<env>.json` + `core/settings.py`

## مستندات بیشتر

راهنمای کامل Python (Router، Config، Middleware، Async):

**https://fusion.cipherunit.xyz/en/docs/python/v1**

## مجوز

MIT
