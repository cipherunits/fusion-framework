using System.Text.Json;
using System.Text.Json.Nodes;

namespace FusionFramework;

/// <summary>HTML handlers using Tera templates (Python/Node parity).</summary>
public abstract class FusionBaseTemplate : FusionBaseApi
{
    public static string Template { get; set; } = "";
    public static string TemplateAddress { get; set; } = "";
    public static string TemplatesDir { get; set; } = "";

    public virtual Dictionary<string, JsonNode?> Context() => new();

    public virtual object Get()
    {
        if (WantsJson())
            return Context();
        return Render();
    }

    public virtual string TemplateName()
    {
        var name = !string.IsNullOrEmpty(Template) ? Template : TemplateAddress;
        if (string.IsNullOrEmpty(name))
            throw new InvalidOperationException($"{GetType().Name} must set Template or TemplateAddress");
        return name;
    }

    public virtual string TemplatesRoot()
    {
        if (!string.IsNullOrEmpty(TemplatesDir))
            return TemplatesDir;
        var fromSettings = SettingsStore.Current.Get("templates.dir", "templates");
        return fromSettings is JsonValue v && v.TryGetValue<string>(out var s) ? s : "templates";
    }

    public virtual object Render(
        int status = 200,
        IDictionary<string, string>? headers = null,
        IDictionary<string, JsonNode?>? context = null,
        string? templateName = null)
    {
        var ctx = new Dictionary<string, JsonNode?>(Context(), StringComparer.Ordinal);
        if (context != null)
        {
            foreach (var kv in context)
                ctx[kv.Key] = kv.Value;
        }
        var html = Templates.Render(
            templateName ?? TemplateName(),
            ctx,
            TemplatesRoot());
        var hdrs = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase)
        {
            ["content-type"] = "text/html; charset=utf-8",
        };
        if (headers != null)
        {
            foreach (var kv in headers)
                hdrs[kv.Key] = kv.Value;
        }
        return Response(html, status, hdrs);
    }
}
