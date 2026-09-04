// Enable the built-in cache monitor via fusion.<env>.json.
// When cache.monitor.enabled is true, FusionApp.Mount registers HTML + /json.
// When false, no routes are registered.

using System.Text.Json.Nodes;
using FusionFramework;

var settings = new FusionSettings();
settings.Merge(new JsonObject
{
    ["cache"] = new JsonObject
    {
        ["driver"] = "moka",
        ["default_ttl"] = null,
        ["monitor"] = new JsonObject
        {
            ["enabled"] = true,
            ["path"] = "/__fusion/cache",
            ["max_events"] = 50,
        },
    },
});

Cache.Configure(settings);
Cache.Set("demo:user", new { name = "Ada" }, 60);
var snap = Cache.Snapshot();
Console.WriteLine($"driver={snap["driver"]} keys={snap["entry_count"]} events={snap["event_count"]}");
Console.WriteLine($"open http://127.0.0.1:8080{snap["monitor"]?["path"]} after Listen()");

// using var app = new FusionApp(settings);
// app.Listen();
