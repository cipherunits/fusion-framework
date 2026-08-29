using System.Text.Json.Nodes;

namespace FusionFramework;

/// <summary>Built-in security / CORS / cache / request-id middleware (settings-driven).</summary>
public static class BuiltinMiddleware
{
    public static IReadOnlyList<FusionMiddleware> FromSettings(FusionSettings settings)
    {
        var outList = new List<FusionMiddleware>();
        var root = AsObject(settings.Get("middleware", new { }));

        if (Enabled(root, settings, "security", defaultEnabled: true, out var securityCfg))
            outList.Add(SecurityHeaders(securityCfg));
        if (Enabled(root, settings, "cors", defaultEnabled: false, out var corsCfg))
            outList.Add(Cors(corsCfg));
        if (Enabled(root, settings, "cache", defaultEnabled: false, out var cacheCfg))
            outList.Add(CacheHeaders(cacheCfg));
        if (Enabled(root, settings, "request_id", defaultEnabled: true, out var ridCfg))
            outList.Add(RequestId(ridCfg));

        return outList;
    }

    public static FusionMiddleware SecurityHeaders(JsonObject? config = null)
    {
        var cfg = config ?? new JsonObject();
        var headers = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase)
        {
            ["X-Content-Type-Options"] = Str(cfg, "content_type_options", "x_content_type_options") ?? "nosniff",
            ["X-Frame-Options"] = Str(cfg, "frame_options", "x_frame_options") ?? "DENY",
            ["Referrer-Policy"] = Str(cfg, "referrer_policy") ?? "strict-origin-when-cross-origin",
            ["Permissions-Policy"] = Str(cfg, "permissions_policy")
                ?? "camera=(), microphone=(), geolocation=(), payment=()",
            ["X-XSS-Protection"] = Str(cfg, "xss_protection") ?? "0",
            ["Cross-Origin-Opener-Policy"] = Str(cfg, "coop") ?? "same-origin",
            ["Cross-Origin-Resource-Policy"] = Str(cfg, "corp") ?? "same-origin",
        };

        var csp = Str(cfg, "csp", "content_security_policy");
        if (!string.IsNullOrEmpty(csp))
            headers["Content-Security-Policy"] = csp;

        var hsts = cfg["hsts"] as JsonObject ?? new JsonObject();
        if (Truthy(hsts["enabled"], false))
        {
            var maxAge = hsts["max_age"]?.GetValue<long?>() ?? 31536000;
            var parts = new List<string> { $"max-age={maxAge}" };
            if (Truthy(hsts["include_subdomains"], true)) parts.Add("includeSubDomains");
            if (Truthy(hsts["preload"], false)) parts.Add("preload");
            headers["Strict-Transport-Security"] = string.Join("; ", parts);
        }

        if (cfg["headers"] is JsonObject overrides)
        {
            foreach (var kv in overrides)
            {
                if (kv.Value is null || kv.Value.GetValueKind() == System.Text.Json.JsonValueKind.Null)
                    headers.Remove(kv.Key);
                else
                    headers[kv.Key] = kv.Value.ToString();
            }
        }

        return (request, callNext) =>
        {
            var result = Middleware.ResolveAwaitable(callNext(request));
            return Middleware.MergeResponseHeaders(result, headers);
        };
    }

    public static FusionMiddleware Cors(JsonObject? config = null)
    {
        var cfg = config ?? new JsonObject();
        var allowOrigins = AsList(cfg["allow_origins"] ?? cfg["origins"], "*");
        var allowMethods = AsList(
            cfg["allow_methods"],
            "GET", "POST", "PUT", "PATCH", "DELETE", "OPTIONS", "HEAD");
        var allowHeaders = AsList(
            cfg["allow_headers"],
            "Authorization", "Content-Type", "Accept", "Origin", "X-Request-Id");
        var exposeHeaders = AsList(cfg["expose_headers"], "X-Request-Id");
        var allowCredentials = Truthy(cfg["allow_credentials"], false);
        var maxAge = cfg["max_age"]?.GetValue<long?>() ?? 600;

        Dictionary<string, string>? Build(string? origin)
        {
            if (allowOrigins.Count == 0) return null;
            var wildcard = allowOrigins.Contains("*");
            string allow;
            if (!string.IsNullOrEmpty(origin) && (wildcard || allowOrigins.Contains(origin)))
                allow = wildcard && !allowCredentials ? "*" : origin!;
            else if (wildcard && !allowCredentials)
                allow = "*";
            else
                return null;

            var headers = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase)
            {
                ["Access-Control-Allow-Origin"] = allow,
                ["Access-Control-Allow-Methods"] = string.Join(", ", allowMethods),
                ["Access-Control-Allow-Headers"] = string.Join(", ", allowHeaders),
                ["Access-Control-Max-Age"] = maxAge.ToString(),
                ["Vary"] = "Origin",
            };
            if (exposeHeaders.Count > 0)
                headers["Access-Control-Expose-Headers"] = string.Join(", ", exposeHeaders);
            if (allowCredentials)
                headers["Access-Control-Allow-Credentials"] = "true";
            return headers;
        }

        return (request, callNext) =>
        {
            request.Headers.TryGetValue("Origin", out var origin);
            if (string.IsNullOrEmpty(origin))
            {
                foreach (var kv in request.Headers)
                {
                    if (kv.Key.Equals("Origin", StringComparison.OrdinalIgnoreCase))
                    {
                        origin = kv.Value;
                        break;
                    }
                }
            }

            var corsHeaders = Build(origin);
            if (request.Method.Equals("OPTIONS", StringComparison.OrdinalIgnoreCase))
            {
                if (corsHeaders is null)
                {
                    return new Dictionary<string, object?>
                    {
                        ["status"] = 403,
                        ["body"] = new Dictionary<string, object?> { ["detail"] = "CORS origin not allowed" },
                    };
                }
                return new Dictionary<string, object?>
                {
                    ["status"] = 204,
                    ["body"] = "",
                    ["headers"] = corsHeaders,
                };
            }

            var result = Middleware.ResolveAwaitable(callNext(request));
            return corsHeaders is null ? result : Middleware.MergeResponseHeaders(result, corsHeaders);
        };
    }

    public static FusionMiddleware CacheHeaders(JsonObject? config = null)
    {
        var cfg = config ?? new JsonObject();
        var value = Str(cfg, "default", "cache_control") ?? "no-store";
        var extra = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase)
        {
            ["Cache-Control"] = value,
        };
        var pragma = Str(cfg, "pragma");
        if (!string.IsNullOrEmpty(pragma))
            extra["Pragma"] = pragma!;
        else if (value == "no-store")
            extra["Pragma"] = "no-cache";

        return (request, callNext) =>
        {
            var result = Middleware.ResolveAwaitable(callNext(request));
            return Middleware.MergeResponseHeaders(result, extra);
        };
    }

    public static FusionMiddleware RequestId(JsonObject? config = null)
    {
        var cfg = config ?? new JsonObject();
        var headerName = Str(cfg, "header") ?? "X-Request-Id";
        var acceptIncoming = Truthy(cfg["incoming"], true);
        var stateKey = Str(cfg, "state_key") ?? "request_id";

        return (request, callNext) =>
        {
            string? rid = null;
            if (acceptIncoming)
            {
                foreach (var kv in request.Headers)
                {
                    if (kv.Key.Equals(headerName, StringComparison.OrdinalIgnoreCase))
                    {
                        rid = kv.Value;
                        break;
                    }
                }
            }
            rid ??= Guid.NewGuid().ToString("N");
            Middleware.EnsureState(request)[stateKey] = JsonValue.Create(rid);

            var result = Middleware.ResolveAwaitable(callNext(request));
            return Middleware.MergeResponseHeaders(result, new Dictionary<string, string>
            {
                [headerName] = rid,
            });
        };
    }

    static bool Enabled(
        JsonObject root,
        FusionSettings settings,
        string name,
        bool defaultEnabled,
        out JsonObject cfg)
    {
        cfg = root[name] as JsonObject
              ?? AsObject(settings.Get(name, new { }))
              ?? new JsonObject();
        if (cfg.ContainsKey("enabled"))
            return Truthy(cfg["enabled"], defaultEnabled);
        return defaultEnabled;
    }

    static JsonObject AsObject(JsonNode? value) =>
        value as JsonObject ?? new JsonObject();

    static string? Str(JsonObject cfg, params string[] keys)
    {
        foreach (var key in keys)
        {
            var node = cfg[key];
            if (node is null || node.GetValueKind() == System.Text.Json.JsonValueKind.Null)
                continue;
            var s = node.ToString();
            if (!string.IsNullOrWhiteSpace(s)) return s;
        }
        return null;
    }

    static bool Truthy(JsonNode? value, bool defaultValue)
    {
        if (value is null || value.GetValueKind() == System.Text.Json.JsonValueKind.Null)
            return defaultValue;
        if (value.GetValueKind() == System.Text.Json.JsonValueKind.True) return true;
        if (value.GetValueKind() == System.Text.Json.JsonValueKind.False) return false;
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

    static List<string> AsList(JsonNode? value, params string[] fallback)
    {
        if (value is JsonArray arr)
            return arr.Select(n => n?.ToString() ?? "").Where(s => s.Length > 0).ToList();
        if (value is JsonValue v && v.TryGetValue<string>(out var s) && !string.IsNullOrWhiteSpace(s))
            return s.Split(',').Select(p => p.Trim()).Where(p => p.Length > 0).ToList();
        return fallback.ToList();
    }
}
