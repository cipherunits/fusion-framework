using System.Text.Json.Nodes;

namespace FusionFramework;

/// <summary>
/// Built-in UI component gallery at <c>/__fusion/component</c> (when assets exist on disk).
/// </summary>
public static class FusionUi
{
    public const string DefaultComponentPath = "/__fusion/component";

    /// <summary>Register gallery HTML + static assets when the fusion templates folder is found.</summary>
    public static bool Mount(FusionApp app, FusionSettings settings)
    {
        var root = ResolveAssetsRoot();
        if (root is null)
            return false;

        var gallery = Path.Combine(root, "components.html");
        if (!File.Exists(gallery))
            return false;

        var path = DefaultComponentPath;
        var raw = AsString(settings.Get("ui.component_path", null));
        if (!string.IsNullOrWhiteSpace(raw))
            path = NormalizePath(raw);

        app.AddRawRoute("GET", path, () => FileResponse(gallery, "text/html; charset=utf-8"));
        if (path != "/")
            app.AddRawRoute("GET", $"{path}/", () => FileResponse(gallery, "text/html; charset=utf-8"));

        var skip = new HashSet<string>(StringComparer.OrdinalIgnoreCase)
        {
            "components.html",
            "index.html",
            "monitor.html",
            "cache_monitor.html",
        };

        foreach (var file in Directory.EnumerateFiles(root, "*", SearchOption.AllDirectories))
        {
            var rel = Path.GetRelativePath(root, file).Replace('\\', '/');
            if (skip.Contains(rel))
                continue;
            var url = $"{path}/{rel}";
            var contentType = GuessContentType(file);
            var captured = file;
            app.AddRawRoute("GET", url, () => FileResponse(captured, contentType));
        }

        return true;
    }

    /// <summary>Find crates/fusion-core/assets/templates/fusion relative to the process.</summary>
    static string? ResolveAssetsRoot()
    {
        var candidates = new List<string>();
        var cwd = Directory.GetCurrentDirectory();
        candidates.Add(Path.Combine(cwd, "crates", "fusion-core", "assets", "templates", "fusion"));
        candidates.Add(Path.Combine(cwd, "..", "crates", "fusion-core", "assets", "templates", "fusion"));

        var asm = typeof(FusionUi).Assembly.Location;
        if (!string.IsNullOrEmpty(asm))
        {
            var dir = Path.GetDirectoryName(asm);
            if (!string.IsNullOrEmpty(dir))
            {
                candidates.Add(Path.GetFullPath(Path.Combine(dir, "..", "..", "..", "..", "..", "crates", "fusion-core", "assets", "templates", "fusion")));
            }
        }

        foreach (var candidate in candidates)
        {
            try
            {
                var full = Path.GetFullPath(candidate);
                if (File.Exists(Path.Combine(full, "components.html")))
                    return full;
            }
            catch
            {
                // ignore invalid paths
            }
        }
        return null;
    }

    static object FileResponse(string filePath, string contentType)
    {
        var bytes = File.ReadAllBytes(filePath);
        return new Dictionary<string, object?>
        {
            ["status"] = 200,
            ["headers"] = new Dictionary<string, string> { ["content-type"] = contentType },
            ["body"] = bytes,
        };
    }

    static string GuessContentType(string filePath)
    {
        var ext = Path.GetExtension(filePath).ToLowerInvariant();
        return ext switch
        {
            ".html" => "text/html; charset=utf-8",
            ".css" => "text/css; charset=utf-8",
            ".js" => "application/javascript; charset=utf-8",
            ".svg" => "image/svg+xml",
            ".png" => "image/png",
            _ => "application/octet-stream",
        };
    }

    static string NormalizePath(string raw)
    {
        var path = string.IsNullOrWhiteSpace(raw) ? DefaultComponentPath : raw.Trim();
        if (!path.StartsWith('/')) path = "/" + path;
        path = path.TrimEnd('/');
        return string.IsNullOrEmpty(path) ? DefaultComponentPath : path;
    }

    static string? AsString(object? value) =>
        value switch
        {
            null => null,
            JsonNode n when n.GetValueKind() == System.Text.Json.JsonValueKind.String => n.GetValue<string>(),
            JsonNode n => n.ToJsonString().Trim('"'),
            _ => value.ToString(),
        };
}
