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
}
