using System.Text.Json.Nodes;
using FusionFramework;
using Xunit;

namespace FusionFramework.Tests;

public class CacheTests
{
    public CacheTests()
    {
        Cache.Reset();
    }

    [Fact]
    public void SetGetDeleteExists()
    {
        Assert.Null(Cache.Get("k"));
        Cache.Set("k", new { n = 1 });
        Assert.True(Cache.Exists("k"));
        Assert.Equal(1, Cache.Get("k")?["n"]?.GetValue<int>());
        Assert.True(Cache.Delete("k"));
        Assert.False(Cache.Exists("k"));
    }

    [Fact]
    public void GetOrSetExistsOrSetDeleteOrSet()
    {
        Assert.Equal(1, Cache.GetOrSet("counter", 1)?.GetValue<int>());
        Assert.Equal(1, Cache.GetOrSet("counter", 99)?.GetValue<int>());
        Assert.False(Cache.ExistsOrSet("flag", true));
        Assert.True(Cache.ExistsOrSet("flag", false));
        Assert.True(Cache.Get("flag")!.GetValue<bool>());
        Assert.Equal("next", Cache.DeleteOrSet("flag", "next")?.GetValue<string>());
        Assert.Equal("moka", Cache.Driver());
    }

    [Fact]
    public void ClearRemovesAll()
    {
        Cache.Set("a", 1);
        Cache.Set("b", 2);
        Cache.Clear();
        Assert.Null(Cache.Get("a"));
        Assert.Null(Cache.Get("b"));
    }

    [Fact]
    public async Task AsyncSetGetClearAndGetOrSetFactory()
    {
        await Cache.SetAsync("async-k", new { ok = true });
        Assert.True(Cache.Get("async-k")?["ok"]?.GetValue<bool>());
        Assert.True(await Cache.ExistsAsync("async-k"));

        var calls = 0;
        var first = await Cache.GetOrSetAsync("ax", async () =>
        {
            calls += 1;
            await Task.Yield();
            return new { v = calls };
        });
        Assert.Equal(1, first?["v"]?.GetValue<int>());
        var second = await Cache.GetOrSetAsync("ax", async () =>
        {
            calls += 1;
            return new { v = calls };
        });
        Assert.Equal(1, second?["v"]?.GetValue<int>());
        Assert.Equal(1, calls);

        await Cache.ClearAsync();
        Assert.Null(await Cache.GetAsync("async-k"));
    }

    [Fact]
    public void SnapshotAndPanelContext()
    {
        Cache.Set("demo", new { n = 1 });
        var snap = Cache.Snapshot();
        Assert.Equal("moka", snap["driver"]?.GetValue<string>());
        Assert.Equal(1, snap["entry_count"]?.GetValue<int>());
        Assert.Equal("demo", snap["entries"]?[0]?["key"]?.GetValue<string>());
        Assert.Equal("set", snap["events"]?[0]?["op"]?.GetValue<string>());

        var ctx = Cache.PanelContext();
        Assert.Equal("Cache Monitor", ctx["title"]?.GetValue<string>());
        Assert.False(ctx["empty_entries"]!.GetValue<bool>());
        Assert.Equal("demo", ctx["entry_rows"]?[0]?[0]?.GetValue<string>());
        Assert.EndsWith("/json", ctx["json_path"]!.GetValue<string>());
    }

    [Fact]
    public void MountRespectsEnabledFlag()
    {
        var off = new FusionSettings();
        off.Merge(new JsonObject
        {
            ["cache"] = new JsonObject
            {
                ["monitor"] = new JsonObject { ["enabled"] = false },
            },
        });
        using var appOff = new FusionApp(off);
        Assert.False(CacheMonitor.Mount(appOff, off));

        var on = new FusionSettings();
        on.Merge(new JsonObject
        {
            ["cache"] = new JsonObject
            {
                ["driver"] = "moka",
                ["monitor"] = new JsonObject
                {
                    ["enabled"] = true,
                    ["path"] = "/__fusion/cache",
                },
            },
        });
        using var appOn = new FusionApp(on);
        Assert.True(CacheMonitor.Mount(appOn, on));
    }
}
