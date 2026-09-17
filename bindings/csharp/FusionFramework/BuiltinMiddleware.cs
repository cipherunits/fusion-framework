using System.Text.Json.Nodes;

namespace FusionFramework;

/// <summary>Convenience aliases for built-in middleware factories used in scaffolded apps.</summary>
public static class BuiltinMiddleware
{
    public static FusionMiddleware FrameworkHeaders() => Middleware.FrameworkHeaders();
    public static FusionMiddleware SecurityHeaders() => Middleware.SecurityHeaders();

    /// <summary>
    /// CORS middleware. When called with no args, reads <c>middleware.cors.*</c> from
    /// <c>fusion.&lt;env&gt;.json</c> (origins/methods/headers). Opting in via
    /// <c>app.Use(BuiltinMiddleware.Cors())</c> always enables CORS even if
    /// <c>middleware.cors.enabled</c> is false in JSON.
    /// </summary>
    public static FusionMiddleware Cors()
    {
        var settings = SettingsStore.Current;
        return Middleware.Cors(
            allowOrigins: ReadStringList(settings, "middleware.cors.allow_origins") ?? new[] { "*" },
            allowMethods: ReadStringList(settings, "middleware.cors.allow_methods"),
            allowHeaders: ReadStringList(settings, "middleware.cors.allow_headers"),
            exposeHeaders: ReadStringList(settings, "middleware.cors.expose_headers"),
            allowCredentials: Truthy(settings.Get("middleware.cors.allow_credentials", false)),
            maxAge: ReadInt(settings, "middleware.cors.max_age", 600));
    }

    public static FusionMiddleware CacheHeaders() => Middleware.CacheHeaders();
    public static FusionMiddleware RequestId() => Middleware.RequestId();
    public static FusionMiddleware StaticFiles(
        string root = "static",
        string prefix = "/static",
        int? maxAge = 3600,
        bool? fallthrough = null) =>
        Middleware.StaticFiles(root, prefix, maxAge, fallthrough);

    static IEnumerable<string>? ReadStringList(FusionSettings settings, string key)
    {
        var node = settings.Get(key, null);
        if (node is not JsonArray arr || arr.Count == 0)
            return null;

        var list = new List<string>();
        foreach (var item in arr)
        {
            var s = item?.GetValue<string>();
            if (!string.IsNullOrWhiteSpace(s))
                list.Add(s);
        }

        return list.Count == 0 ? null : list;
    }

    static int ReadInt(FusionSettings settings, string key, int fallback)
    {
        var node = settings.Get(key, fallback);
        if (node is JsonValue v && v.TryGetValue<int>(out var n))
            return n;
        return fallback;
    }

    static bool Truthy(JsonNode? node, bool fallback = false)
    {
        if (node is null) return fallback;
        if (node is JsonValue v)
        {
            if (v.TryGetValue<bool>(out var b)) return b;
            if (v.TryGetValue<string>(out var s))
                return s is "1" or "true" or "True" or "yes" or "on";
        }
        return fallback;
    }
}
