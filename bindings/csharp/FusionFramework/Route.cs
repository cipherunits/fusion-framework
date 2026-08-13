using System.Reflection;
using System.Runtime.InteropServices;
using System.Text.Json;
using System.Text.Json.Nodes;

namespace FusionFramework;

[AttributeUsage(AttributeTargets.Class, AllowMultiple = false, Inherited = false)]
public sealed class RouteAttribute : Attribute
{
    public string Path { get; }
    public string[]? Tags { get; set; }
    public string? Desc { get; set; }
    public string? Title { get; set; }
    public string? Version { get; set; }
    public bool Deprecated { get; set; }
    public string[]? Roles { get; set; }
    public string RoleClaim { get; set; } = "roles";
    public string RoleStateKey { get; set; } = "jwt";

    public RouteAttribute(string path) => Path = path;
}

internal sealed class RouteEntry
{
    public required string Path { get; init; }
    public required Type ApiClass { get; init; }
    public List<FusionMiddleware> Middleware { get; init; } = new();
}

/// <summary>Route registry — mirrors Python <c>route</c> / Node <c>router</c>.</summary>
public static class Route
{
    static readonly List<RouteEntry> Registry = new();
    static readonly object Gate = new();

    public static string ResolvePath(string template, string className) =>
        Native.TakeUtf8(Native.fusion_resolve_route_path(template, className));

    public static string ApiResourceName(string className) =>
        Native.TakeUtf8(Native.fusion_api_resource_name(className));

    public static Type Register(
        Type apiClass,
        string path,
        IEnumerable<FusionMiddleware>? middleware = null,
        IEnumerable<string>? roles = null,
        string? version = null,
        string roleClaim = "roles",
        string roleStateKey = "jwt")
    {
        if (!typeof(FusionBaseApi).IsAssignableFrom(apiClass))
            throw new ArgumentException($"{apiClass.Name} must extend FusionBaseApi");

        var resolved = ResolvePath(path, apiClass.Name);
        var v = (version ?? "").Trim();
        if (v.Length > 0)
            resolved = $"{v.TrimStart('/')}/{resolved.TrimStart('/')}";

        var chain = middleware?.ToList() ?? new List<FusionMiddleware>();
        if (roles != null)
        {
            var roleList = roles.ToList();
            if (roleList.Count > 0)
                chain.Add(Middleware.RequireRoles(roleList, roleClaim, roleStateKey));
        }

        lock (Gate)
        {
            Registry.Add(new RouteEntry
            {
                Path = resolved.StartsWith('/') ? resolved : "/" + resolved,
                ApiClass = apiClass,
                Middleware = chain,
            });
        }
        return apiClass;
    }

    public static Type Register<T>() where T : FusionBaseApi
    {
        var attr = typeof(T).GetCustomAttribute<RouteAttribute>()
                   ?? throw new InvalidOperationException($"{typeof(T).Name} needs [Route(...)]");
        return Register(
            typeof(T),
            attr.Path,
            roles: attr.Roles,
            version: attr.Version,
            roleClaim: attr.RoleClaim,
            roleStateKey: attr.RoleStateKey);
    }

    public static void RegisterAll(Assembly assembly)
    {
        foreach (var type in assembly.GetTypes())
        {
            if (!typeof(FusionBaseApi).IsAssignableFrom(type) || type.IsAbstract)
                continue;
            if (type.GetCustomAttribute<RouteAttribute>() is null)
                continue;
            // re-entrancy safe: Register reads attribute again
            var attr = type.GetCustomAttribute<RouteAttribute>()!;
            Register(
                type,
                attr.Path,
                roles: attr.Roles,
                version: attr.Version,
                roleClaim: attr.RoleClaim,
                roleStateKey: attr.RoleStateKey);
        }
    }

    internal static List<RouteEntry> Snapshot()
    {
        lock (Gate) return Registry.ToList();
    }

    public static void Clear()
    {
        lock (Gate) Registry.Clear();
    }
}

internal sealed class RouteSlot
{
    public required RouteEntry Entry { get; init; }
    public required string Method { get; init; }
    public required MethodInfo Handler { get; init; }
    public required IReadOnlyList<FusionMiddleware> Global { get; init; }
    public Native.FusionHandlerFn? KeptAlive { get; set; }
}

internal static class JsonUtil
{
    public static Dictionary<string, string> ParseStringMap(string? json)
    {
        var map = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
        if (string.IsNullOrWhiteSpace(json)) return map;
        using var doc = JsonDocument.Parse(json);
        foreach (var p in doc.RootElement.EnumerateObject())
            map[p.Name] = p.Value.ValueKind == JsonValueKind.String
                ? p.Value.GetString() ?? ""
                : p.Value.ToString();
        return map;
    }

    public static Dictionary<string, JsonNode?> ParseState(string? json)
    {
        var map = new Dictionary<string, JsonNode?>(StringComparer.Ordinal);
        if (string.IsNullOrWhiteSpace(json)) return map;
        var node = JsonNode.Parse(json) as JsonObject;
        if (node is null) return map;
        foreach (var kv in node)
            map[kv.Key] = kv.Value;
        return map;
    }

    public static string SerializeResponse(object? result)
    {
        if (result is null)
            return """{"status":200,"body":""}""";
        if (result is string s)
            return JsonSerializer.Serialize(new { status = 200, body = s });
        if (result is HttpException ex)
            return JsonSerializer.Serialize(ex.ToResponse());

        // Already an envelope or plain object
        return JsonSerializer.Serialize(result);
    }
}
