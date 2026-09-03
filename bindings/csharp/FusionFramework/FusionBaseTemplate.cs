using System.Text.Json;
using System.Text.Json.Nodes;

namespace FusionFramework;

/// <summary>HTML handlers using Tera templates (Python/Node parity).</summary>
public abstract class FusionBaseTemplate : FusionBaseApi
{
    public static string Template { get; set; } = "";
    public static string TemplateAddress { get; set; } = "";
    public static string TemplatesDir { get; set; } = "";

    /// <summary>Sync template variables (override in subclasses).</summary>
    public virtual Dictionary<string, JsonNode?> Context() => new();

    /// <summary>Async template variables; default wraps <see cref="Context"/>.</summary>
    public virtual Task<Dictionary<string, JsonNode?>> ContextAsync() =>
        Task.FromResult(Context());

    /// <summary>
    /// Default GET — HTML or JSON context. Uses <see cref="ContextAsync"/> so
    /// subclasses can override that for DB/API-backed pages (Python async context parity).
    /// </summary>
    public virtual object Get()
    {
        var task = ContextAsync();
        if (task.IsCompletedSuccessfully)
            return FinishGet(task.Result);
        return FinishGetAsync(task);
    }

    async Task<object> FinishGetAsync(Task<Dictionary<string, JsonNode?>> task)
    {
        var ctx = await task.ConfigureAwait(false);
        return FinishGet(ctx);
    }

    object FinishGet(Dictionary<string, JsonNode?> ctx)
    {
        if (WantsJson())
            return ctx;
        return HtmlResponse(ctx);
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
        var task = ContextAsync();
        if (!task.IsCompletedSuccessfully)
            return RenderAsync(task, status, headers, context, templateName);

        var ctx = new Dictionary<string, JsonNode?>(task.Result, StringComparer.Ordinal);
        if (context != null)
        {
            foreach (var kv in context)
                ctx[kv.Key] = kv.Value;
        }
        return HtmlResponse(ctx, status, headers, templateName);
    }

    async Task<object> RenderAsync(
        Task<Dictionary<string, JsonNode?>> task,
        int status,
        IDictionary<string, string>? headers,
        IDictionary<string, JsonNode?>? context,
        string? templateName)
    {
        var ctx = new Dictionary<string, JsonNode?>(await task.ConfigureAwait(false), StringComparer.Ordinal);
        if (context != null)
        {
            foreach (var kv in context)
                ctx[kv.Key] = kv.Value;
        }
        return HtmlResponse(ctx, status, headers, templateName);
    }

    /// <summary>Render an already-resolved context dictionary to an HTML envelope.</summary>
    protected object HtmlResponse(
        IDictionary<string, JsonNode?> ctx,
        int status = 200,
        IDictionary<string, string>? headers = null,
        string? templateName = null)
    {
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
