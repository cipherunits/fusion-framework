namespace FusionFramework;

/// <summary>Shared content negotiation helpers (fusion-core parity).</summary>
internal static class ContentNegotiation
{
    public static bool PrefersJson(string? accept, string? formatQuery)
    {
        if (string.Equals(formatQuery, "json", StringComparison.OrdinalIgnoreCase))
            return true;

        accept = accept?.Trim();
        if (string.IsNullOrEmpty(accept))
            return false;

        var bestJson = -1.0f;
        var bestHtml = -1.0f;

        foreach (var part in accept.Split(','))
        {
            var tokens = part.Trim().Split(';', StringSplitOptions.TrimEntries | StringSplitOptions.RemoveEmptyEntries);
            var media = tokens.Length > 0 ? tokens[0].ToLowerInvariant() : "";
            var q = 1.0f;
            for (var i = 1; i < tokens.Length; i++)
            {
                if (tokens[i].StartsWith("q=", StringComparison.OrdinalIgnoreCase)
                    && float.TryParse(tokens[i][2..], out var parsed))
                {
                    q = parsed;
                }
            }

            switch (media)
            {
                case "application/json":
                case "text/json":
                    bestJson = Math.Max(bestJson, q);
                    break;
                case "text/html":
                case "application/xhtml+xml":
                    bestHtml = Math.Max(bestHtml, q);
                    break;
            }
        }

        return bestJson > 0 && bestJson >= bestHtml;
    }
}
