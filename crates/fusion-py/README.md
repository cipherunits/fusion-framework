# Fusion Framework (Python)

Class-based HTTP APIs on a shared Rust core (`fusion-core`).

## Install

```bash
pip install fusion-framework
```

Scaffold with [Fusion Tool](https://github.com/cipherunits/fusion-tool):

```bash
fusion init --lang python --name my-app
```

## Quick start

```python
from fusion_framework.api import FusionBaseApi
from fusion_framework.route import route
from fusion_framework import status
from fusion_framework.app import FusionApp
from fusion_framework.config import get_settings, load_settings_module


@route("api/[module]/{id}")
class ItemModule(FusionBaseApi):
    def get(self, id: int):
        return self.response({"id": id}, status=status.HTTP_SUCCESS)


MIDDLEWARE: list = []  # optional — no defaults


def main() -> None:
    load_settings_module("settings")
    app = FusionApp(get_settings())
    for mw in MIDDLEWARE:
        app.use(mw)
    app.listen()


if __name__ == "__main__":
    main()
```

## Docs

Full guides (router, config, middleware, async):  
**https://fusion.cipherunit.xyz/en/docs/python/v1**

- Site: [fusion.cipherunit.xyz](https://fusion.cipherunit.xyz/)
- GitHub: [cipherunits/fusion-framework](https://github.com/cipherunits/fusion-framework)

## License

BSD 3-Clause
