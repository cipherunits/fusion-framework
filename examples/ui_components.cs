// Render Fusion Tera UI components (button, badge, table + page_size).
// Run from repo root after building the C# binding.

using System.Text.Json.Nodes;
using FusionFramework;

var root = Path.Combine(Directory.GetCurrentDirectory(), "examples", "ui_components_assets");

var ctx = new Dictionary<string, JsonNode?>
{
    ["headers"] = new JsonArray("Route", "Method"),
    ["rows"] = new JsonArray(
        new JsonArray("/v1/api/product", "GET"),
        new JsonArray("/swagger", "GET"),
        new JsonArray("/__fusion/cache", "GET"),
        new JsonArray("/health", "GET")),
};

var html = Templates.Render("page.html", ctx, root);
Console.WriteLine(html);
