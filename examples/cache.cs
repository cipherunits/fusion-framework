// Application cache demo (sync + async, default driver: moka).

using FusionFramework;

Cache.Reset();
Cache.Set("greeting", new { hello = "world" }, ttlSeconds: 30);
Console.WriteLine($"get: {Cache.Get("greeting")}");
Console.WriteLine($"exists: {Cache.Exists("greeting")}");
Console.WriteLine($"getOrSet: {Cache.GetOrSet("counter", 1)}");
Console.WriteLine($"existsOrSet (first): {Cache.ExistsOrSet("flag", true)}");
Console.WriteLine($"existsOrSet (again): {Cache.ExistsOrSet("flag", false)}");
Console.WriteLine($"deleteOrSet: {Cache.DeleteOrSet("greeting", new { hello = "fusion" })}");
Console.WriteLine($"driver: {Cache.Driver()}");
Cache.Clear();
Console.WriteLine($"after clear: {Cache.Get("greeting")}");

await Cache.SetAsync("async-greeting", new { hello = "async" }, ttlSeconds: 30);
Console.WriteLine($"aget: {await Cache.GetAsync("async-greeting")}");
Console.WriteLine(
    $"agetOrSet: {await Cache.GetOrSetAsync("async-counter", async () =>
    {
        await Task.Yield();
        return 1;
    })}");
await Cache.ClearAsync();
Console.WriteLine($"after aclear: {await Cache.GetAsync("async-greeting")}");
