using System.Text.Json;
using System.Text.Json.Nodes;

namespace FusionFramework;

/// <summary>Process-wide application cache (default driver: moka).</summary>
public static class Cache
{
    static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNamingPolicy = null,
    };

    /// <summary>Apply <c>cache.*</c> from a settings handle.</summary>
    public static void Configure(FusionSettings settings)
    {
        if (Native.fusion_cache_configure(settings.Handle) != 0)
            throw new InvalidOperationException("cache configure failed");
    }

    /// <summary>Ensure a default moka cache is ready.</summary>
    public static void Ensure()
    {
        if (Native.fusion_cache_ensure() != 0)
            throw new InvalidOperationException("cache ensure failed");
    }

    /// <summary>Store a JSON-compatible value.
    /// <paramref name="ttlSeconds"/> null → use settings <c>default_ttl</c>; if that is also null, no expiry.</summary>
    public static void Set(string key, object? value, double? ttlSeconds = null)
    {
        Ensure();
        var json = JsonSerializer.Serialize(value, JsonOptions);
        if (Native.fusion_cache_set(key, json, ttlSeconds ?? -1.0) != 0)
            throw new InvalidOperationException($"cache set failed for key '{key}'");
    }

    /// <summary>Return the cached value, or <c>null</c> if missing/expired.</summary>
    public static JsonNode? Get(string key)
    {
        Ensure();
        var ptr = Native.fusion_cache_get(key);
        if (ptr == IntPtr.Zero)
            return null;
        var json = Native.TakeUtf8(ptr);
        return string.IsNullOrEmpty(json) ? null : JsonNode.Parse(json);
    }

    /// <summary>Remove a key; returns whether it existed.</summary>
    public static bool Delete(string key)
    {
        Ensure();
        var code = Native.fusion_cache_delete(key);
        if (code < 0)
            throw new InvalidOperationException($"cache delete failed for key '{key}'");
        return code == 1;
    }

    /// <summary>True when the key is present and not expired.</summary>
    public static bool Exists(string key)
    {
        Ensure();
        var code = Native.fusion_cache_exists(key);
        if (code < 0)
            throw new InvalidOperationException($"cache exists failed for key '{key}'");
        return code == 1;
    }

    /// <summary>Return cached value, or store <paramref name="defaultValue"/> and return it.</summary>
    public static JsonNode? GetOrSet(string key, object? defaultValue, double? ttlSeconds = null)
    {
        Ensure();
        var json = JsonSerializer.Serialize(defaultValue, JsonOptions);
        var ptr = Native.fusion_cache_get_or_set(key, json, ttlSeconds ?? -1.0);
        var raw = Native.TakeUtf8(ptr);
        return string.IsNullOrEmpty(raw) ? null : JsonNode.Parse(raw);
    }

    /// <summary>Delete then set; returns the stored value.</summary>
    public static JsonNode? DeleteOrSet(string key, object? value, double? ttlSeconds = null)
    {
        Ensure();
        var json = JsonSerializer.Serialize(value, JsonOptions);
        var ptr = Native.fusion_cache_delete_or_set(key, json, ttlSeconds ?? -1.0);
        var raw = Native.TakeUtf8(ptr);
        return string.IsNullOrEmpty(raw) ? null : JsonNode.Parse(raw);
    }

    /// <summary>If key exists return <c>true</c>; otherwise set value and return <c>false</c>.</summary>
    public static bool ExistsOrSet(string key, object? value, double? ttlSeconds = null)
    {
        Ensure();
        var json = JsonSerializer.Serialize(value, JsonOptions);
        var code = Native.fusion_cache_exists_or_set(key, json, ttlSeconds ?? -1.0);
        if (code < 0)
            throw new InvalidOperationException($"cache exists_or_set failed for key '{key}'");
        return code == 1;
    }

    /// <summary>Remove every entry from the process-wide cache.</summary>
    public static void Clear()
    {
        Ensure();
        if (Native.fusion_cache_clear() != 0)
            throw new InvalidOperationException("cache clear failed");
    }

    /// <summary>Active driver name (e.g. <c>moka</c>).</summary>
    public static string Driver()
    {
        Ensure();
        var ptr = Native.fusion_cache_driver();
        var name = Native.TakeUtf8(ptr);
        if (string.IsNullOrEmpty(name))
            throw new InvalidOperationException("cache driver lookup failed");
        return name;
    }

    /// <summary>Drop the global cache instance (tests).</summary>
    public static void Reset() => Native.fusion_cache_reset();

    /// <summary>JSON snapshot of live entries and recent mutations (monitor).</summary>
    public static JsonNode Snapshot()
    {
        Ensure();
        var ptr = Native.fusion_cache_snapshot();
        var json = Native.TakeUtf8(ptr);
        if (string.IsNullOrEmpty(json))
            throw new InvalidOperationException("cache snapshot failed");
        return JsonNode.Parse(json) ?? new JsonObject();
    }

    /// <summary>Template context for the built-in <c>fusion/cache_monitor.html</c> panel.</summary>
    public static JsonNode PanelContext()
    {
        Ensure();
        var ptr = Native.fusion_cache_panel_context();
        var json = Native.TakeUtf8(ptr);
        if (string.IsNullOrEmpty(json))
            throw new InvalidOperationException("cache panel_context failed");
        return JsonNode.Parse(json) ?? new JsonObject();
    }

    /// <summary>Async <see cref="Set"/>.</summary>
    public static Task SetAsync(string key, object? value, double? ttlSeconds = null) =>
        Task.Run(() => Set(key, value, ttlSeconds));

    /// <summary>Async <see cref="Get"/>.</summary>
    public static Task<JsonNode?> GetAsync(string key) =>
        Task.Run(() => Get(key));

    /// <summary>Async <see cref="Delete"/>.</summary>
    public static Task<bool> DeleteAsync(string key) =>
        Task.Run(() => Delete(key));

    /// <summary>Async <see cref="Exists"/>.</summary>
    public static Task<bool> ExistsAsync(string key) =>
        Task.Run(() => Exists(key));

    /// <summary>Async <see cref="GetOrSet"/> with a sync default value.</summary>
    public static Task<JsonNode?> GetOrSetAsync(
        string key,
        object? defaultValue,
        double? ttlSeconds = null) =>
        Task.Run(() => GetOrSet(key, defaultValue, ttlSeconds));

    /// <summary>Async <see cref="GetOrSet"/>; factory may be async.</summary>
    public static async Task<JsonNode?> GetOrSetAsync(
        string key,
        Func<Task<object?>> factory,
        double? ttlSeconds = null)
    {
        if (await ExistsAsync(key).ConfigureAwait(false))
            return await GetAsync(key).ConfigureAwait(false);
        var value = await factory().ConfigureAwait(false);
        await SetAsync(key, value, ttlSeconds).ConfigureAwait(false);
        return await GetAsync(key).ConfigureAwait(false);
    }

    /// <summary>Async <see cref="DeleteOrSet"/>.</summary>
    public static Task<JsonNode?> DeleteOrSetAsync(
        string key,
        object? value,
        double? ttlSeconds = null) =>
        Task.Run(() => DeleteOrSet(key, value, ttlSeconds));

    /// <summary>Async <see cref="ExistsOrSet"/>.</summary>
    public static Task<bool> ExistsOrSetAsync(
        string key,
        object? value,
        double? ttlSeconds = null) =>
        Task.Run(() => ExistsOrSet(key, value, ttlSeconds));

    /// <summary>Async <see cref="Clear"/>.</summary>
    public static Task ClearAsync() => Task.Run(Clear);
}
