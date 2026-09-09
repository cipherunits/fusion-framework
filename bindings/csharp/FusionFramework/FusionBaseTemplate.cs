using System.Text.Json;
using System.Text.Json.Nodes;

namespace FusionFramework;

/// <summary>HTML handlers using Tera templates (Python/Node parity).</summary>
public abstract class FusionBaseTemplate : FusionBaseApi
{
    public static string Template { get; set; } = "";
    public static string TemplateAddress { get; set; } = "";
    public static string TemplatesDir { get; set; } = "";

    /// <summary>
    /// Template variables (not an HTTP verb). Override in subclasses.
    /// <see cref="Get"/> renders this as HTML; POST handlers use <see cref="Form"/> / <see cref="Ok"/> / <see cref="Fail"/>.
    /// </summary>
    public virtual Dictionary<string, JsonNode?> Context() => new();

    /// <summary>Async template variables; default wraps <see cref="Context"/>.</summary>
    public virtual Task<Dictionary<string, JsonNode?>> ContextAsync() =>
        Task.FromResult(Context());

    /// <summary>Parsed POST body (urlencoded or JSON) as flat string fields.</summary>
    public Dictionary<string, string> Form => ParseFormBody(Body, ContentType());

    /// <summary>Parse urlencoded or JSON body into flat string fields.</summary>
    public static Dictionary<string, string> ParseFormBody(string? body, string? contentType)
    {
        var raw = body ?? "";
        var ct = (contentType ?? "").ToLowerInvariant();
        var outDict = new Dictionary<string, string>(StringComparer.Ordinal);
        if (ct.Contains("application/json", StringComparison.Ordinal)
            || (raw.TrimStart().StartsWith('{') && !ct.Contains("urlencoded", StringComparison.Ordinal)))
        {
            try
            {
                var node = string.IsNullOrWhiteSpace(raw) ? null : JsonNode.Parse(raw);
                if (node is JsonObject obj)
                {
                    foreach (var kv in obj)
                        outDict[kv.Key] = kv.Value is null || kv.Value.GetValueKind() == JsonValueKind.Null
                            ? ""
                            : kv.Value.ToString() ?? "";
                }
            }
            catch (JsonException)
            {
                // ignore invalid JSON
            }
            return outDict;
        }

        if (string.IsNullOrEmpty(raw))
            return outDict;

        foreach (var pair in raw.Split('&', StringSplitOptions.RemoveEmptyEntries))
        {
            var parts = pair.Split('=', 2);
            var key = Uri.UnescapeDataString(parts[0].Replace('+', ' '));
            var value = parts.Length > 1
                ? Uri.UnescapeDataString(parts[1].Replace('+', ' '))
                : "";
            outDict[key] = value;
        }
        return outDict;
    }

    string? ContentType()
    {
        foreach (var kv in Request.Headers)
        {
            if (string.Equals(kv.Key, "Content-Type", StringComparison.OrdinalIgnoreCase))
                return kv.Value;
        }
        return null;
    }

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

    /// <summary>Validation failure — JSON for SPA fetch, else same template with errors.</summary>
    public object Fail(
        IDictionary<string, string>? errors = null,
        string? message = null,
        IDictionary<string, string>? fields = null)
    {
        var err = errors?.ToDictionary(kv => kv.Key, kv => kv.Value, StringComparer.Ordinal)
                  ?? new Dictionary<string, string>(StringComparer.Ordinal);
        var flat = fields?.ToDictionary(
                       kv => kv.Key,
                       kv => kv.Value ?? "",
                       StringComparer.Ordinal)
                   ?? new Dictionary<string, string>(StringComparer.Ordinal);
        var msg = message ?? "Validation failed";
        if (WantsJson())
        {
            return Response(new Dictionary<string, object?>
            {
                ["ok"] = false,
                ["message"] = msg,
                ["errors"] = err,
                ["fields"] = flat,
            }, 400);
        }
        return FormHtmlResult(ok: false, message: msg, errors: err, fields: flat, status: 400);
    }

    /// <summary>Success — JSON for SPA fetch, else same template with ok=true.</summary>
    public object Ok(string? message = null, IDictionary<string, string>? fields = null)
    {
        var flat = fields?.ToDictionary(
                       kv => kv.Key,
                       kv => kv.Value ?? "",
                       StringComparer.Ordinal)
                   ?? new Dictionary<string, string>(StringComparer.Ordinal);
        var msg = message ?? "OK";
        if (WantsJson())
        {
            return Response(new Dictionary<string, object?>
            {
                ["ok"] = true,
                ["message"] = msg,
                ["errors"] = new Dictionary<string, string>(),
                ["fields"] = flat,
            }, 200);
        }
        return FormHtmlResult(
            ok: true,
            message: msg,
            errors: new Dictionary<string, string>(),
            fields: flat,
            status: 200);
    }

    object FormHtmlResult(
        bool ok,
        string message,
        IDictionary<string, string> errors,
        IDictionary<string, string> fields,
        int status)
    {
        var task = ContextAsync();
        if (!task.IsCompletedSuccessfully)
            return FormHtmlResultAsync(task, ok, message, errors, fields, status);
        return FinishFormHtml(task.Result, ok, message, errors, fields, status);
    }

    async Task<object> FormHtmlResultAsync(
        Task<Dictionary<string, JsonNode?>> task,
        bool ok,
        string message,
        IDictionary<string, string> errors,
        IDictionary<string, string> fields,
        int status)
    {
        var ctx = await task.ConfigureAwait(false);
        return FinishFormHtml(ctx, ok, message, errors, fields, status);
    }

    object FinishFormHtml(
        Dictionary<string, JsonNode?> ctx,
        bool ok,
        string message,
        IDictionary<string, string> errors,
        IDictionary<string, string> fields,
        int status)
    {
        var data = new Dictionary<string, JsonNode?>(ctx, StringComparer.Ordinal);
        foreach (var kv in fields)
            data[kv.Key] = JsonValue.Create(kv.Value);
        data["ok"] = JsonValue.Create(ok);
        data["message"] = JsonValue.Create(message);
        data["errors"] = JsonSerializer.SerializeToNode(errors);
        data["fields"] = JsonSerializer.SerializeToNode(fields);
        return HtmlResponse(data, status);
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
