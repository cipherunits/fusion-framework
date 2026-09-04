// Built-in Fusion monitor (cache + background tasks).
// Enable via fusion.<env>.json:
//   "monitor": { "enabled": true, "path": "/__fusion/monitor" }
//   "cache": { "max_events": 50 }

using System.Text.Json.Nodes;
using FusionFramework;

var settings = new FusionSettings();
settings.Merge(new JsonObject
{
    ["monitor"] = new JsonObject
    {
        ["enabled"] = true,
        ["path"] = "/__fusion/monitor",
    },
    ["cache"] = new JsonObject
    {
        ["driver"] = "moka",
        ["default_ttl"] = null,
        ["max_events"] = 50,
    },
});

Cache.Configure(settings);
Cache.Set("demo:user", new { name = "Ada" }, 60);

BackgroundTasks.Reset();
BackgroundTasks.Spawn(() => { });
BackgroundTasks.SpawnAfter(5000, () => { });

var snap = Cache.Snapshot();
var tasks = snap["tasks"]!;
Console.WriteLine($"driver={snap["driver"]} keys={snap["entry_count"]} events={snap["event_count"]}");
Console.WriteLine($"tasks active={tasks["active_count"]} total={tasks["task_count"]}");
Console.WriteLine($"open http://127.0.0.1:8080{snap["monitor"]?["path"]} after Listen()");
