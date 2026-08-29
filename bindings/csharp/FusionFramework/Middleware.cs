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

    /// <summary>Default identity middleware — advertises Fusion on every response.</summary>
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
        var suppressed = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        if (result is Dictionary<string, object?> probe
            && probe.TryGetValue("suppress_headers", out var raw)
            && raw is IEnumerable<string> names)
        {
            foreach (var n in names) suppressed.Add(n);
        }

        if (result is Dictionary<string, object?> dict)
        {
            var headers = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
            foreach (var kv in extra)
            {
                if (!suppressed.Contains(kv.Key))
                    headers[kv.Key] = kv.Value;
            }
            if (dict.TryGetValue("headers", out var existing) && existing is IDictionary<string, string> map)
            {
                foreach (var kv in map) headers[kv.Key] = kv.Value;
            }
            dict["headers"] = headers;
            return dict;
        }

        var seeded = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
        foreach (var kv in extra)
        {
            if (!suppressed.Contains(kv.Key))
                seeded[kv.Key] = kv.Value;
        }
        return new Dictionary<string, object?>
        {
            ["status"] = 200,
            ["body"] = result,
            ["headers"] = seeded,
        };
    }

    static object Error(int status, string detail) =>
        new Dictionary<string, object?>
        {
            ["status"] = status,
            ["body"] = new Dictionary<string, object?> { ["detail"] = detail },
        };

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
