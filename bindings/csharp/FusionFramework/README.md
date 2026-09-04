# Fusion Framework (C#)

Minimal managed bindings over the Rust core via `fusion-ffi`. DX mirrors Python/Node: `FusionApp`, `[Route]`, middleware, `Status`, `Settings`, Swagger.

`/swagger` is on by default. Route `Version` values (for example `v1`) each get their own OpenAPI spec and appear in the Swagger navbar.

## Links

- **Docs:** [fusion.cipherunit.xyz](https://fusion.cipherunit.xyz/)
- **GitHub:** [cipherunits/fusion-framework](https://github.com/cipherunits/fusion-framework)
- **CLI:** [cipherunits/fusion-tool](https://github.com/cipherunits/fusion-tool)

## Prerequisites

Requires .NET 8+ (this package targets `net10.0` by default; retarget as needed).

```bash
dotnet add package Fusion-Framework
```

NuGet package id is `Fusion-Framework`; the C# namespace remains `FusionFramework`.

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

[Route("api/[module]", Version = "v1", Tags = new[] { "products" })]
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

### Auto-reload (development)

```csharp
// Restart the process when source files change
app.Listen(reload: true);

// Never reload (default) — same as omit / settings reload: false
app.Listen(reload: false);
```

Or in `fusion.dev.json`:

```json
{ "reload": true }
```

## Custom HTTP routes

Use method-level attributes alongside convention `get`/`post`/… handlers:

```csharp
[Route("/api/[module]")]
public class UserModule : FusionBaseApi
{
    public object Get() => Response(new { mode = "convention" });

    [HttpGet("test/[action]")]
    public object UserAction() => Response(new { mode = "custom" });
}
```

## Pagination

List endpoints can use `PageParams()` / `Paginated()` on `FusionBaseApi`:

```csharp
public object Get()
{
    var p = PageParams(page: 2, pageSize: 10);
    var items = FetchItems((int)p.Offset, (int)p.Limit);
    return Paginated(items, total: 100, p);
}
```

Query params: `page`, `page_size` (aliases: `per_page`, `limit`), optional `offset`.

`[action]` becomes the handler name without an `Action` suffix, lowercased (`UserAction` → `user`).

## Handler parameters

Handler method parameters are bound automatically (Python parity):

- **GET** (and other reads): from query string — `Get(string name)` → `?name=alice`
- **POST/PUT/PATCH**: from JSON body — `Post(string name)` → `{ "name": "alice" }`
- **Path tokens** `{id}`: from route params

Parameters appear in Swagger OpenAPI docs.

```csharp
MIDDLEWARE.Add(Middleware.BearerJwt());
Route.Register(typeof(AdminModule), "/api/admin", permissions: new[] { AdminChecks.IsAdmin });
```

## Static files

Serve CSS/images without custom routes (WhiteNoise-style).

- **root**: folder on disk (e.g. `static`)
- **prefix**: URL prefix (e.g. `/static`)

So `static/logo.png` is available at `/static/logo.png`:

```csharp
app.Use(Middleware.StaticFiles(root: "static", prefix: "/static"));
// <img src="/static/logo.png">
```

Use `prefix: "/"` when files should be served at the site root
(`templates/home/a.png` → `/a.png`). Files are mounted as real routes on
`FusionApp.Mount()` / `Listen()`.

## Cache

Process-wide cache (default driver **moka**). Configure via `cache` in `fusion.<env>.json`.

```csharp
Cache.Set("user:1", new { name = "Ada" }, ttlSeconds: 60);
var value = Cache.Get("user:1");
Cache.GetOrSet("counter", 1);
Cache.ExistsOrSet("flag", true);
Cache.DeleteOrSet("user:1", new { name = "Bob" });
Cache.Clear();
await Cache.SetAsync("user:2", new { name = "Ada" });
await Cache.ClearAsync();

## Background tasks

```csharp
var id = BackgroundTasks.Spawn(() => Console.WriteLine("done"));
BackgroundTasks.SpawnAfter(1000, () => Console.WriteLine("later"));
BackgroundTasks.Cancel(id);
BackgroundTasks.Status(id); // pending|running|done|cancelled|failed
```
```

## License

BSD 3-Clause
