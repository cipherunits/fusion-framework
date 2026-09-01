using System.Text.Json;
using System.Text.Json.Nodes;

namespace FusionFramework;

/// <summary>Tera template rendering via fusion-ffi.</summary>
public static class Templates
{
    public static string Render(
        string templateName,
        IDictionary<string, JsonNode?>? context = null,
        string? templatesRoot = null)
    {
        var ctxJson = context is null || context.Count == 0
            ? "{}"
            : JsonSerializer.Serialize(context);
        var root = templatesRoot ?? "templates";
        var ptr = Native.fusion_render_template(templateName, ctxJson, root);
        var html = Native.TakeUtf8(ptr);
        if (string.IsNullOrEmpty(html))
            throw new InvalidOperationException($"template render failed: {templateName}");
        return html;
    }
}
