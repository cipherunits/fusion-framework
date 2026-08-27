// Paginated list API example.
//
// GET /v1/api/product?page=2&page_size=10

using FusionFramework;

[Route("api/[module]", Version = "v1", Tags = new[] { "products" })]
public class ProductModule : FusionBaseApi
{
    static readonly object[] AllProducts = Enumerable.Range(1, 50)
        .Select(i => new { id = i, name = $"product-{i}" })
        .ToArray();

    public object Get()
    {
        var p = PageParams();
        var start = (int)p.Offset;
        var end = Math.Min(start + (int)p.Limit, AllProducts.Length);
        var items = AllProducts[start..end];
        return Paginated(items, (ulong)AllProducts.Length, p, status: Status.HTTP_SUCCESS);
    }
}

// Route.Register<ProductModule>();
// SettingsStore.Current.EnsureLoaded(Directory.GetCurrentDirectory());
// using var app = new FusionApp(SettingsStore.GetSettings());
// app.Listen();
