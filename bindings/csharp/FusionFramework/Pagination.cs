using System.Text.Json;
using System.Text.Json.Nodes;

namespace FusionFramework;

/// <summary>Normalized pagination state for list APIs (fusion-core parity).</summary>
public sealed class PaginationParams
{
    public ulong Page { get; init; }
    public ulong PageSize { get; init; }
    public ulong Offset { get; init; }

    public ulong Limit => PageSize;

    public ulong TotalPages(ulong total) =>
        PageSize == 0 ? 0 : (total + PageSize - 1) / PageSize;

    public bool HasNext(ulong total) => Offset + PageSize < total;

    public bool HasPrev() => Page > 1;
}

/// <summary>Parse and build paginated list responses.</summary>
public static class Pagination
{
    public static PaginationParams Parse(
        IReadOnlyDictionary<string, string> query,
        ulong? page = null,
        ulong? pageSize = null,
        ulong? offset = null,
        ulong defaultPageSize = 20,
        ulong maxPageSize = 100)
    {
        var merged = new Dictionary<string, string>(query, StringComparer.Ordinal);
        if (page is not null) merged["page"] = page.Value.ToString();
        if (pageSize is not null) merged["page_size"] = pageSize.Value.ToString();
        if (offset is not null) merged["offset"] = offset.Value.ToString();
        return ParseQuery(merged, defaultPageSize, maxPageSize);
    }

    public static JsonObject Body(object items, ulong total, PaginationParams p)
    {
        var totalPages = p.TotalPages(total);
        return new JsonObject
        {
            ["items"] = JsonSerializer.SerializeToNode(items),
            ["pagination"] = new JsonObject
            {
                ["page"] = p.Page,
                ["page_size"] = p.PageSize,
                ["offset"] = p.Offset,
                ["limit"] = p.Limit,
                ["total"] = total,
                ["total_pages"] = totalPages,
                ["has_next"] = p.HasNext(total),
                ["has_prev"] = p.HasPrev(),
            },
        };
    }

    static PaginationParams ParseQuery(
        IReadOnlyDictionary<string, string> query,
        ulong defaultPageSize,
        ulong maxPageSize)
    {
        var page = ParseU64(query, "page") ?? 1;
        if (page == 0)
            throw new PaginationException("page must be >= 1");

        var rawSize = FirstParseU64(query, "page_size", "per_page", "limit");
        var pageSize = rawSize ?? defaultPageSize;
        if (pageSize == 0)
            throw new PaginationException("page_size must be >= 1");
        if (pageSize > maxPageSize)
            pageSize = maxPageSize;

        var offset = ParseU64(query, "offset") ?? (page - 1) * pageSize;

        return new PaginationParams
        {
            Page = page,
            PageSize = pageSize,
            Offset = offset,
        };
    }

    static ulong? ParseU64(IReadOnlyDictionary<string, string> query, string key)
    {
        if (!query.TryGetValue(key, out var raw) || string.IsNullOrWhiteSpace(raw))
            return null;
        return ulong.TryParse(raw.Trim(), out var n) ? n : null;
    }

    static ulong? FirstParseU64(IReadOnlyDictionary<string, string> query, params string[] keys)
    {
        foreach (var key in keys)
        {
            var n = ParseU64(query, key);
            if (n is not null)
                return n;
        }
        return null;
    }
}

public sealed class PaginationException : Exception
{
    public PaginationException(string message) : base(message) { }
}
