using System.Text;
using System.Text.Json;
using System.Text.Json.Nodes;

namespace FusionFramework;

public delegate object? FusionMiddleware(FusionRequest request, Func<FusionRequest, object?> callNext);

public static class Middleware
{
    static readonly List<FusionMiddleware> ActiveGlobal = new();

    public static void SetActiveGlobal(IEnumerable<FusionMiddleware> middlewares)
    {
        ActiveGlobal.Clear();
        ActiveGlobal.AddRange(middlewares);
    }

    public static IReadOnlyList<FusionMiddleware> GetActiveGlobal() => ActiveGlobal.ToList();

    public static void ClearActiveGlobal() => ActiveGlobal.Clear();

    public static object? RunChain(
        FusionRequest request,
        IReadOnlyList<FusionMiddleware> middlewares,
        Func<FusionRequest, object?> handler)
    {
        EnsureState(request);

        object? Dispatch(int index, FusionRequest req)
        {
            if (index >= middlewares.Count)
                return handler(req);

            var mw = middlewares[index];
            return mw(req, next => Dispatch(index + 1, next));
        }

        // Sync FFI callback: unwrap Task/ValueTask from async handlers / middleware.
        return ResolveAwaitable(Dispatch(0, request));
    }

    /// <summary>
    /// Unwrap nested <see cref="Task"/> / <see cref="ValueTask"/> results (async API methods).
    /// Mirrors Python awaiting coroutines before treating the value as an HTTP body.
    /// </summary>
    public static object? ResolveAwaitable(object? result)
    {
        for (var depth = 0; depth < 8; depth++)
        {
            switch (result)
            {
                case null:
                    return null;
                case Task task:
                    {
                        task.ConfigureAwait(false).GetAwaiter().GetResult();
                        var t = task.GetType();
                        if (t.IsGenericType)
                        {
                            result = t.GetProperty("Result")?.GetValue(task);
                            continue;
                        }
                        return null;
                    }
                default:
                    {
                        var type = result.GetType();
                        if (type == typeof(ValueTask))
                        {
                            ((ValueTask)result).ConfigureAwait(false).GetAwaiter().GetResult();
                            return null;
                        }
                        if (type.IsGenericType && type.GetGenericTypeDefinition() == typeof(ValueTask<>))
                        {
                            // ValueTask<T> → Task<T> then unwrap on next iteration.
                            result = type.GetMethod("AsTask")!.Invoke(result, null);
                            continue;
                        }
                        return result;
                    }
            }
        }

        throw new InvalidOperationException("Awaitable nesting too deep");
    }

    public static Dictionary<string, JsonNode?> EnsureState(FusionRequest request)
    {
        return request.State;
    }

    /// <summary>Bearer JWT middleware. Optional <paramref name="verify"/>; otherwise base64url-decodes payload (no crypto).</summary>
    public static FusionMiddleware BearerJwt(
        Func<string, JsonObject?>? verify = null,
        string stateKey = "jwt",
        string header = "Authorization")
    {
        return (request, callNext) =>
        {
            request.Headers.TryGetValue(header, out var auth);
            if (string.IsNullOrEmpty(auth))
            {
                foreach (var kv in request.Headers)
                {
                    if (string.Equals(kv.Key, header, StringComparison.OrdinalIgnoreCase))
                    {
                        auth = kv.Value;
                        break;
                    }
                }
            }

            if (string.IsNullOrEmpty(auth) || !auth.StartsWith("Bearer ", StringComparison.OrdinalIgnoreCase))
                return Error(401, "Missing bearer token");

            var token = auth["Bearer ".Length..].Trim();
            try
            {
                JsonObject? payload;
                if (verify != null)
                {
                    payload = verify(token);
                    if (payload is null)
                        return Error(401, "Invalid token");
                }
                else
                {
                    var parts = token.Split('.');
                    if (parts.Length != 3) throw new InvalidOperationException("bad token");
                    var json = Encoding.UTF8.GetString(Base64UrlDecode(parts[1]));
                    payload = JsonNode.Parse(json) as JsonObject
                              ?? throw new InvalidOperationException("bad payload");
                }

                EnsureState(request)[stateKey] = payload;
                return callNext(request);
            }
            catch
            {
                return Error(401, "Invalid token");
            }
        };
    }

    public static FusionMiddleware RequireRoles(
        IEnumerable<string> roles,
        string claim = "roles",
        string stateKey = "jwt")
    {
        var allowed = new HashSet<string>(roles.Select(r => r.ToString()), StringComparer.Ordinal);
        return (request, callNext) =>
        {
            if (!EnsureState(request).TryGetValue(stateKey, out var payload) || payload is null)
                return Error(401, "Authentication required");

            var claimNode = payload[claim];
            if (claimNode is null)
                return Error(403, $"Missing '{claim}' claim");

            List<string> userRoles;
            if (claimNode is JsonArray arr)
                userRoles = arr.Select(n => n?.ToString() ?? "").ToList();
            else if (claimNode is JsonValue)
                userRoles = new List<string> { claimNode.ToString() };
            else
                return Error(403, $"Invalid '{claim}' claim");

            if (!userRoles.Any(r => allowed.Contains(r)))
            {
                return new Dictionary<string, object?>
                {
                    ["status"] = 403,
                    ["body"] = new Dictionary<string, object?>
                    {
                        ["detail"] = "Insufficient permissions",
                        ["required"] = allowed.ToList(),
                    },
                };
            }

            return callNext(request);
        };
    }

    public static FusionMiddleware RequireRoles(params string[] roles) =>
        RequireRoles((IEnumerable<string>)roles);

    /// <summary>Common security response headers.</summary>
    public static FusionMiddleware SecurityHeaders(
        string contentTypeOptions = "nosniff",
        string frameOptions = "DENY",
        string referrerPolicy = "strict-origin-when-cross-origin",
        string permissionsPolicy = "camera=(), microphone=(), geolocation=(), payment=()",
        string coop = "same-origin",
        string corp = "same-origin",
        string? csp = null,
        string? hsts = null)
    {
        var extra = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase)
        {
            ["X-Content-Type-Options"] = contentTypeOptions,
            ["X-Frame-Options"] = frameOptions,
            ["Referrer-Policy"] = referrerPolicy,
            ["Permissions-Policy"] = permissionsPolicy,
            ["Cross-Origin-Opener-Policy"] = coop,
            ["Cross-Origin-Resource-Policy"] = corp,
        };
        if (!string.IsNullOrEmpty(csp)) extra["Content-Security-Policy"] = csp!;
        if (!string.IsNullOrEmpty(hsts)) extra["Strict-Transport-Security"] = hsts!;

        return (request, callNext) =>
        {
            var result = ResolveAwaitable(callNext(request));
            return MergeResponseHeaders(result, extra);
        };
    }

    /// <summary>Set <c>Cache-Control</c> on responses.</summary>
    public static FusionMiddleware CacheHeaders(string defaultValue = "no-store") =>
        (request, callNext) =>
        {
            var result = ResolveAwaitable(callNext(request));
            return MergeResponseHeaders(result, new Dictionary<string, string>
            {
                ["Cache-Control"] = defaultValue,
            });
        };

    /// <summary>Echo or generate <c>X-Request-Id</c> on each request.</summary>
    public static FusionMiddleware RequestId(string header = "X-Request-Id", bool incoming = true) =>
        (request, callNext) =>
        {
            string? rid = incoming ? GetHeader(request, header) : null;
            if (string.IsNullOrEmpty(rid))
                rid = Guid.NewGuid().ToString();
            EnsureState(request)["request_id"] = rid;
            var result = ResolveAwaitable(callNext(request));
            return MergeResponseHeaders(result, new Dictionary<string, string> { [header] = rid });
        };

    /// <summary>CORS middleware; answers <c>OPTIONS</c> preflight with 204.</summary>
    public static FusionMiddleware Cors(
        IEnumerable<string>? allowOrigins = null,
        IEnumerable<string>? allowMethods = null,
        IEnumerable<string>? allowHeaders = null,
        IEnumerable<string>? exposeHeaders = null,
        bool allowCredentials = false,
        int maxAge = 600)
    {
        var origins = (allowOrigins ?? new[] { "*" }).Select(o => o.ToString()).ToList();
        var methods = (allowMethods ?? new[]
        {
            "GET", "POST", "PUT", "PATCH", "DELETE", "OPTIONS", "HEAD",
        }).Select(m => m.ToUpperInvariant()).ToList();
        var headers = (allowHeaders ?? new[]
        {
            "Authorization", "Content-Type", "Accept", "Origin", "X-Request-Id",
        }).ToList();
        var expose = (exposeHeaders ?? new[] { "X-Request-Id" }).ToList();
        var allowAll = origins.Contains("*");

        Dictionary<string, string> CorsHeaders(string? origin)
        {
            var chosen = "*";
            if (!allowAll)
            {
                if (!string.IsNullOrEmpty(origin) && origins.Contains(origin))
                    chosen = origin;
                else if (origins.Count > 0)
                    chosen = origins[0];
            }

            var map = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase)
            {
                ["Access-Control-Allow-Origin"] = chosen,
                ["Access-Control-Allow-Methods"] = string.Join(", ", methods),
                ["Access-Control-Allow-Headers"] = string.Join(", ", headers),
                ["Access-Control-Expose-Headers"] = string.Join(", ", expose),
                ["Access-Control-Max-Age"] = maxAge.ToString(),
                ["Vary"] = "Origin",
            };
            if (allowCredentials && chosen != "*")
                map["Access-Control-Allow-Credentials"] = "true";
            return map;
        }

        return (request, callNext) =>
        {
            var origin = GetHeader(request, "Origin");
            var extra = CorsHeaders(origin);
            if (string.Equals(request.Method, "OPTIONS", StringComparison.OrdinalIgnoreCase))
            {
                return new Dictionary<string, object?>
                {
                    ["status"] = 204,
                    ["body"] = "",
                    ["headers"] = extra,
                };
            }

            var result = ResolveAwaitable(callNext(request));
            return MergeResponseHeaders(result, extra);
        };
    }

    /// <summary>Optional identity middleware — not enabled by default. Add via <c>app.Use(Middleware.FrameworkHeaders())</c>.</summary>
    public static FusionMiddleware FrameworkHeaders()
    {
        var extra = Header.Fingerprint();
        return (request, callNext) =>
        {
            // Must resolve awaitables first — otherwise Task becomes the JSON body
            // (same class of bug as Python's un-awaited coroutine stringification).
            var result = ResolveAwaitable(callNext(request));
            return MergeResponseHeaders(result, extra);
        };
    }

    internal static object MergeResponseHeaders(object? result, IReadOnlyDictionary<string, string> extra)
    {
        if (result is Dictionary<string, object?> dict)
        {
            var headers = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
            foreach (var kv in extra) headers[kv.Key] = kv.Value;
            if (dict.TryGetValue("headers", out var existing) && existing is IDictionary<string, string> map)
            {
                foreach (var kv in map) headers[kv.Key] = kv.Value;
            }
            dict["headers"] = headers;
            return dict;
        }

        return new Dictionary<string, object?>
        {
            ["status"] = 200,
            ["body"] = result,
            ["headers"] = new Dictionary<string, string>(extra, StringComparer.OrdinalIgnoreCase),
        };
    }

    static object Error(int status, string detail) =>
        new Dictionary<string, object?>
        {
            ["status"] = status,
            ["body"] = new Dictionary<string, object?> { ["detail"] = detail },
        };

    static string? GetHeader(FusionRequest request, string name)
    {
        if (request.Headers.TryGetValue(name, out var direct) && !string.IsNullOrEmpty(direct))
            return direct;
        foreach (var kv in request.Headers)
        {
            if (string.Equals(kv.Key, name, StringComparison.OrdinalIgnoreCase))
                return kv.Value;
        }
        return null;
    }

    static byte[] Base64UrlDecode(string input)
    {
        var s = input.Replace('-', '+').Replace('_', '/');
        switch (s.Length % 4)
        {
            case 2: s += "=="; break;
            case 3: s += "="; break;
        }
        return Convert.FromBase64String(s);
    }
}
