using System.Text.Json;
using System.Text.Json.Nodes;

namespace FusionFramework;

/// <summary>
/// Built-in Fusion monitor (cache + background tasks HTML + JSON).
/// Mounted when <c>monitor.enabled</c> is true (legacy <c>cache.monitor.enabled</c> still works).
/// </summary>
public static class FusionMonitor
{
    /// <summary>Register HTML and <c>/json</c> routes when enabled in settings.</summary>
    public static bool Mount(FusionApp app, FusionSettings settings)
    {
        if (!Enabled(settings))
            return false;

        Cache.Configure(settings);
        var path = ResolvePath(settings);

        app.AddRawRoute("GET", path, () => new MonitorPanel().Get());
        if (path != "/")
            app.AddRawRoute("GET", $"{path}/", () => new MonitorPanel().Get());
        app.AddRawRoute("GET", $"{path}/json", () => Cache.Snapshot());
        return true;
    }

    /// <summary>Prefer <c>monitor.enabled</c>, else legacy <c>cache.monitor.enabled</c>.</summary>
    static bool Enabled(FusionSettings settings)
    {
        var top = settings.Get("monitor.enabled", null);
        if (top is not null && !IsJsonNull(top))
            return Truthy(AsNode(top), false);
        return Truthy(AsNode(settings.Get("cache.monitor.enabled", false)), false);
    }

    /// <summary>Prefer <c>monitor.path</c>, else legacy <c>cache.monitor.path</c>.</summary>
    static string ResolvePath(FusionSettings settings)
    {
        var top = AsString(settings.Get("monitor.path", null));
        if (!string.IsNullOrWhiteSpace(top))
            return NormalizePath(top);
        return NormalizePath(AsString(settings.Get("cache.monitor.path", "/__fusion/monitor")));
    }

    static bool IsJsonNull(object? value) =>
        value is null
        || (value is JsonNode n && n.GetValueKind() == JsonValueKind.Null);

    static JsonNode? AsNode(object? value) =>
        value switch
        {
            null => null,
            JsonNode n => n,
            bool b => JsonValue.Create(b),
            string s => JsonValue.Create(s),
            _ => JsonValue.Create(value.ToString()),
        };

    static string NormalizePath(string? raw)
    {
        var path = string.IsNullOrWhiteSpace(raw) ? "/__fusion/monitor" : raw.Trim();
        if (!path.StartsWith('/')) path = "/" + path;
        path = path.TrimEnd('/');
        return string.IsNullOrEmpty(path) ? "/__fusion/monitor" : path;
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

/// <summary>Backward-compatible alias for <see cref="FusionMonitor"/>.</summary>
public static class CacheMonitor
{
    /// <inheritdoc cref="FusionMonitor.Mount"/>
    public static bool Mount(FusionApp app, FusionSettings settings) =>
        FusionMonitor.Mount(app, settings);
}

/// <summary>Default Fusion monitor page (cache + tasks).</summary>
public sealed class MonitorPanel : FusionBaseTemplate
{
    /// <inheritdoc />
    public override string TemplateName() => "fusion/monitor.html";

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

/// <summary>Backward-compatible alias for <see cref="MonitorPanel"/>.</summary>
public sealed class CacheMonitorPanel : FusionBaseTemplate
{
    /// <inheritdoc />
    public override string TemplateName() => "fusion/monitor.html";

    /// <inheritdoc />
    public override Dictionary<string, JsonNode?> Context() => new MonitorPanel().Context();
}
