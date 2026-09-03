using System.Text.Json.Nodes;
using FusionFramework;
using FusionFramework.Testing;
using Xunit;

namespace FusionFramework.Tests;

public class RouteTests
{
    public RouteTests() => FusionTestSupport.ClearRoutes();

    [Fact]
    public void ResolvePath_expands_module_token()
    {
        var path = Route.ResolvePath("/api/[module]", "ProductModule");
        Assert.Equal("/api/product", path);
    }

    [Fact]
    public void Register_mounts_convention_and_custom_http_slots()
    {
        Route.Register(typeof(ProductModule), "/api/[module]");

        var entry = Assert.Single(Route.Snapshot());
        Assert.Equal("/api/product", entry.ClassBasePath);
        Assert.Contains(entry.Slots, s => s.Path == "/api/product" && s.HttpMethod == "get");
        Assert.Contains(entry.Slots, s => s.Path == "/api/product/catalog/catalog" && s.HttpMethod == "get");
    }

    [Fact]
    public void OpenApi_lists_registered_api_paths()
    {
        Route.Register(typeof(ProductModule), "/api/[module]", tags: new[] { "products" });

        var spec = FusionTestSupport.OpenApiSpec();
        var paths = spec["paths"]!.AsObject();
        Assert.True(paths.ContainsKey("/api/product"));
        Assert.True(paths["/api/product"]!.AsObject().ContainsKey("get"));
    }

    [Fact]
    public void Template_routes_are_omitted_from_openapi()
    {
        Route.Register(typeof(SampleHomePage), "/pages/home");
        Route.Register(typeof(ProductModule), "/api/[module]", version: "v1");

        var combined = FusionTestSupport.OpenApiSpec();
        Assert.False(combined["paths"]!.AsObject().ContainsKey("/pages/home"));

        var v1 = FusionTestSupport.OpenApiSpec("v1");
        Assert.False(v1["paths"]!.AsObject().ContainsKey("/pages/home"));
        Assert.True(v1["paths"]!.AsObject().ContainsKey("/v1/api/product"));
    }

    [Route("/api/[module]")]
    sealed class ProductModule : FusionBaseApi
    {
        public object Get() => Response(new { ok = true });

        [HttpGet("catalog/[action]")]
        public object CatalogAction() => Response(new { items = Array.Empty<object>() });
    }

    [Route("/pages/home")]
    sealed class SampleHomePage : FusionBaseTemplate
    {
        static SampleHomePage()
        {
            Template = "home/index.html";
        }

        public override Dictionary<string, JsonNode?> Context() => new()
        {
            ["title"] = "Home",
        };
    }
}
