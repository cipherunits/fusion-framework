# Fusion Framework (C#)

Minimal managed bindings over the Rust core via `fusion-ffi`. DX mirrors Python/Node: `FusionApp`, `[Route]`, middleware, `Status`, `Settings`.

No full OpenAPI/Swagger in this v1 — runnable HTTP APIs only.

## Links

- **Docs:** [fusion.cipherunit.xyz](https://fusion.cipherunit.xyz/)
- **GitHub:** [cipherunits/fusion-framework](https://github.com/cipherunits/fusion-framework)
- **CLI:** [cipherunits/fusion-tool](https://github.com/cipherunits/fusion-tool)

## Prerequisites

Requires .NET 8+ (this package targets `net10.0` by default; retarget as needed).

```bash
dotnet add package FusionFramework
```

Native `fusion-ffi` is packed under `runtimes/<rid>/native/` in the NuGet package.
For local development from this repo:

```bash
# from repo root
cargo build -p fusion-ffi
# optional: export FUSION_FFI_PATH=$PWD/target/debug/libfusion_ffi.so
```

```bash
dotnet build bindings/csharp/FusionFramework
```

## Quick start

```csharp
using FusionFramework;

[Route("api/[module]")]
public class ProductModule : FusionBaseApi
{
    public object Get() =>
        Response(new { products_id = 12 }, Status.HTTP_SUCCESS);
}

var MIDDLEWARE = new List<FusionMiddleware>(); // none by default

Route.Register<ProductModule>();
SettingsStore.Current.EnsureLoaded(Directory.GetCurrentDirectory());
var app = new FusionApp(SettingsStore.GetSettings());
foreach (var mw in MIDDLEWARE) app.Use(mw);
app.Listen();
```

## Middleware

```csharp
MIDDLEWARE.Add(Middleware.BearerJwt());
Route.Register(typeof(AdminModule), "/api/admin", roles: new[] { "admin" });
```

## License

BSD 3-Clause
