using System.Text.Json;
using System.Text.Json.Nodes;

namespace FusionFramework;

public sealed class FusionRequest
{
    public string Method { get; set; } = "";
    public string Path { get; set; } = "";
    public string Body { get; set; } = "";
    public Dictionary<string, string> Headers { get; set; } = new(StringComparer.OrdinalIgnoreCase);
    public Dictionary<string, string> Params { get; set; } = new(StringComparer.Ordinal);
    public Dictionary<string, string> Query { get; set; } = new(StringComparer.Ordinal);
    public Dictionary<string, JsonNode?> State { get; set; } = new(StringComparer.Ordinal);
}

/// <summary>Base class for class-based route modules (Python/Node parity).</summary>
public abstract class FusionBaseApi
{
    public FusionRequest Request { get; set; } = new();

    public string Method => Request.Method.ToUpperInvariant();
    public string Path => Request.Path;
    public string Body => Request.Body;
    public IReadOnlyDictionary<string, string> Headers => Request.Headers;
    public IReadOnlyDictionary<string, string> Params => Request.Params;
    public IReadOnlyDictionary<string, string> Query => Request.Query;
    public IDictionary<string, JsonNode?> State => Request.State;

    /// <summary>True when the client prefers JSON (Accept header or ?format=json).</summary>
    public bool WantsJson()
    {
        string? accept = null;
        foreach (var kv in Request.Headers)
        {
            if (string.Equals(kv.Key, "Accept", StringComparison.OrdinalIgnoreCase))
            {
                accept = kv.Value;
                break;
            }
        }
        Request.Query.TryGetValue("format", out var format);
        return ContentNegotiation.PrefersJson(accept, format);
    }

    public object Response(object? body = null, int status = 200, IDictionary<string, string>? headers = null)
    {
        var envelope = new Dictionary<string, object?>
        {
            ["status"] = status,
            ["body"] = body switch
            {
                null => "",
                string s => s,
                JsonNode n => n,
                _ => JsonSerializer.SerializeToNode(body),
            },
        };
        if (headers is { Count: > 0 })
            envelope["headers"] = headers.ToDictionary(kv => kv.Key, kv => (object)kv.Value);
        return envelope;
    }

    /// <summary>Read pagination from query (explicit args override query values).</summary>
    public PaginationParams PageParams(
        ulong? page = null,
        ulong? pageSize = null,
        ulong? offset = null,
        ulong defaultPageSize = 20,
        ulong maxPageSize = 100) =>
        Pagination.Parse(
            Request.Query,
            page,
            pageSize,
            offset,
            defaultPageSize,
            maxPageSize);

    /// <summary>Return a paginated list response envelope.</summary>
    public object Paginated(
        object items,
        ulong total,
        PaginationParams? parameters = null,
        ulong? page = null,
        ulong? pageSize = null,
        int status = 200,
        IDictionary<string, string>? headers = null)
    {
        var p = parameters ?? PageParams(page: page, pageSize: pageSize);
        var body = Pagination.Body(items, total, p);
        return Response(body, status, headers);
    }
}
