using System.Text.Json;
using System.Text.Json.Nodes;

namespace FusionFramework;

/// <summary>
/// Built-in process-wide cache monitor (HTML + JSON).
/// Mounted only when <c>cache.monitor.enabled</c> is true — disabled means no endpoints.
/// </summary>
public static class CacheMonitor
{
    /// <summary>Register HTML and <c>/json</c> routes when enabled in settings.</summary>
    public static bool Mount(FusionApp app, FusionSettings settings)
    {
        if (!Truthy(settings.Get("cache.monitor.enabled", false), false))
            return false;

        Cache.Configure(settings);
        var path = NormalizePath(AsString(settings.Get("cache.monitor.path", "/__fusion/cache")));

        app.AddRawRoute("GET", path, () => new CacheMonitorPanel().Get());
        if (path != "/")
            app.AddRawRoute("GET", $"{path}/", () => new CacheMonitorPanel().Get());
        app.AddRawRoute("GET", $"{path}/json", () => Cache.Snapshot());
        return true;
    }

    static string NormalizePath(string? raw)
    {
        var path = string.IsNullOrWhiteSpace(raw) ? "/__fusion/cache" : raw.Trim();
        if (!path.StartsWith('/')) path = "/" + path;
        path = path.TrimEnd('/');
        return string.IsNullOrEmpty(path) ? "/__fusion/cache" : path;
    }

    static bool Truthy(JsonNode? value, bool defaultValue)
    {
        if (value is null || value.GetValueKind() == JsonValueKind.Null)
            return defaultValue;
        if (value.GetValueKind() == JsonValueKind.False) return false;
        if (value.GetValueKind() == JsonValueKind.True) return true;
        if (value is JsonValue v)
        {
            if (v.TryGetValue<bool>(out var b)) return b;
            if (v.TryGetValue<int>(out var n)) return n != 0;
            if (v.TryGetValue<string>(out var s))
            {
                return s.ToLowerInvariant() switch
                {
                    "false" or "0" or "off" or "no" => false,
                    "true" or "1" or "on" or "yes" => true,
                    _ => !string.IsNullOrWhiteSpace(s),
                };
            }
        }
        return true;
    }

    static string? AsString(object? value)
    {
        if (value is null) return null;
        if (value is string s) return s;
        if (value is JsonValue jv && jv.TryGetValue<string>(out var str)) return str;
        if (value is JsonNode node)
            return node.GetValueKind() == JsonValueKind.String ? node.GetValue<string>() : node.ToJsonString();
        return value.ToString();
    }
}

/// <summary>Default cache monitor page (FusionBaseTemplate + built-in components).</summary>
public sealed class CacheMonitorPanel : FusionBaseTemplate
{
    /// <inheritdoc />
    public override string TemplateName() => "fusion/cache_monitor.html";

    /// <inheritdoc />
    public override Dictionary<string, JsonNode?> Context()
    {
        var node = Cache.PanelContext();
        var dict = new Dictionary<string, JsonNode?>(StringComparer.Ordinal);
        if (node is JsonObject obj)
        {
            foreach (var kv in obj)
                dict[kv.Key] = kv.Value?.DeepClone();
        }
        return dict;
    }
}
