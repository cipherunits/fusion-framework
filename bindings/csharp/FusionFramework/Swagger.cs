using System.Reflection;
using System.Text.Json;
using System.Text.Json.Nodes;

namespace FusionFramework;

internal static class SwaggerDocs
{
    const string UnversionedName = "default";
    static readonly string[] HttpMethods = ["get", "post", "put", "patch", "delete", "head", "options"];

    public static void Mount(FusionApp app, FusionSettings settings)
    {
        var swagger = ReadSettings(settings);
        if (swagger is null) return;

        var prefix = swagger.Path;
        var labels = ApplyVersionNavbar(swagger);

        var combined = BuildOpenApi(swagger);
        app.AddRawRoute("GET", $"{prefix}/openapi.json", () => combined);
        app.AddRawRoute("GET", prefix, () => Html(UiHtml(swagger, $"{prefix}/openapi.json")));
        if (prefix != "/")
            app.AddRawRoute("GET", $"{prefix}/", () => Html(UiHtml(swagger, $"{prefix}/openapi.json")));

        foreach (var label in labels)
        {
            var spec = BuildOpenApi(swagger, label);
            app.AddRawRoute("GET", $"{prefix}/{label}/openapi.json", () => spec);
            app.AddRawRoute("GET", $"{prefix}/{label}", () => Html(UiHtml(swagger, $"{prefix}/{label}/openapi.json", label)));
            app.AddRawRoute("GET", $"{prefix}/{label}/", () => Html(UiHtml(swagger, $"{prefix}/{label}/openapi.json", label)));
        }
    }

    static object Html(string body) => new Dictionary<string, object?>
    {
        ["status"] = 200,
        ["body"] = body,
        ["headers"] = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase)
        {
            ["content-type"] = "text/html",
        },
    };

    sealed class SwaggerConfig
    {
        public required string Path { get; init; }
        public required string PageTitle { get; init; }
        public required JsonObject Info { get; init; }
        public JsonArray Servers { get; init; } = new();
        public JsonObject AuthSchemes { get; init; } = new();
        public JsonArray AuthGlobal { get; init; } = new();
        public JsonObject AuthOauth { get; init; } = new();
        public bool NavbarEnabled { get; set; } = true;
        public bool ShowUrlInput { get; set; } = true;
        public bool ShowUrlInputSet { get; init; }
        public bool UrlsSet { get; init; }
        public JsonArray? Urls { get; set; }
        public required JsonObject Ui { get; init; }
    }

    static SwaggerConfig? ReadSettings(FusionSettings settings)
    {
        if (!Truthy(settings.Get("swagger.enabled", true), true))
            return null;

        var pathNode = settings.Get("swagger.path", "/swagger");
        if (IsDisabledPath(pathNode))
            return null;

        var prefix = (AsString(pathNode) ?? "/swagger").TrimEnd('/');
        if (string.IsNullOrEmpty(prefix)) prefix = "/swagger";
        if (!prefix.StartsWith('/')) prefix = "/" + prefix;

        var info = AsObject(settings.Get("swagger.info", new { })) ?? new JsonObject();
        foreach (var key in new[] { "title", "version", "description", "termsOfService", "contact", "license" })
        {
            var flat = settings.Get($"swagger.{key}");
            if (flat is not null && flat.GetValueKind() != JsonValueKind.Null && info[key] is null)
                info[key] = flat.DeepClone();
        }
        info["title"] ??= "fusion-framework";
        info["version"] ??= "1.0.0";

        var pageTitle = AsString(settings.Get("swagger.title"))
            ?? AsString(info["title"])
            ?? "Fusion API Docs";

        var auth = AsObject(settings.Get("swagger.auth", new { })) ?? new JsonObject();
        var persistAuth = Truthy(auth["persistAuthorization"], false);

        var navbar = AsObject(settings.Get("swagger.navbar", new { })) ?? new JsonObject();
        var urlsNode = navbar["urls"];
        var urlsSet = urlsNode is JsonArray;
        JsonArray? urls = urlsSet ? urlsNode as JsonArray : null;

        var ui = new JsonObject
        {
            ["deepLinking"] = true,
            ["displayOperationId"] = false,
            ["defaultModelsExpandDepth"] = 1,
            ["defaultModelExpandDepth"] = 1,
            ["defaultModelRendering"] = "example",
            ["docExpansion"] = "list",
            ["filter"] = true,
            ["tryItOutEnabled"] = true,
            ["persistAuthorization"] = persistAuth,
            ["displayRequestDuration"] = true,
            ["showExtensions"] = false,
            ["showCommonExtensions"] = false,
            ["syntaxHighlight"] = new JsonObject { ["activated"] = true, ["theme"] = "agate" },
            ["withCredentials"] = false,
            ["validatorUrl"] = "https://validator.swagger.io/validator",
        };
        var uiOverlay = AsObject(settings.Get("swagger.ui", new { }));
        if (uiOverlay is not null)
        {
            foreach (var kv in uiOverlay)
                ui[kv.Key] = kv.Value?.DeepClone();
        }
        if (auth.ContainsKey("persistAuthorization"))
            ui["persistAuthorization"] = persistAuth;

        var servers = settings.Get("swagger.servers") as JsonArray ?? new JsonArray();

        return new SwaggerConfig
        {
            Path = prefix,
            PageTitle = pageTitle,
            Info = info,
            Servers = servers,
            AuthSchemes = AsObject(auth["schemes"]) ?? new JsonObject(),
            AuthGlobal = auth["global"] as JsonArray ?? new JsonArray(),
            AuthOauth = AsObject(auth["oauth"]) ?? new JsonObject(),
            NavbarEnabled = Truthy(navbar["enabled"], true),
            ShowUrlInput = Truthy(navbar["showUrlInput"], true),
            ShowUrlInputSet = navbar.ContainsKey("showUrlInput"),
            UrlsSet = urlsSet,
            Urls = urls,
            Ui = ui,
        };
    }

    static List<string> ApplyVersionNavbar(SwaggerConfig swagger)
    {
        var auto = VersionUrls(swagger.Path);
        var labels = auto
            .Select(u => AsString(u?["name"]))
            .Where(name => !string.IsNullOrEmpty(name))
            .Select(name => name!)
            .ToList();
        if (!swagger.UrlsSet && auto.Count > 0)
        {
            swagger.Urls = auto;
            if (!swagger.ShowUrlInputSet)
                swagger.ShowUrlInput = false;
        }
        return labels;
    }

    static JsonArray VersionUrls(string prefix)
    {
        var urls = new JsonArray();
        var seen = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        var hasUnversioned = false;
        foreach (var entry in Route.Snapshot())
        {
            var label = NormalizeVersion(entry.Version);
            if (string.IsNullOrEmpty(label))
            {
                hasUnversioned = true;
                continue;
            }
            if (!seen.Add(label)) continue;
            urls.Add(new JsonObject
            {
                ["url"] = $"{prefix}/{label}/openapi.json",
                ["name"] = label,
            });
        }
        if (hasUnversioned && urls.Count > 0)
        {
            urls.Add(new JsonObject
            {
                ["url"] = $"{prefix}/{UnversionedName}/openapi.json",
                ["name"] = UnversionedName,
            });
        }
        return urls;
    }

    static JsonObject BuildOpenApi(SwaggerConfig swagger, string? version = null)
    {
        var info = swagger.Info.DeepClone().AsObject() ?? new JsonObject();
        var label = NormalizeVersion(version);
        if (!string.IsNullOrEmpty(label) && !label.Equals(UnversionedName, StringComparison.OrdinalIgnoreCase))
            info["version"] = label;

        var spec = new JsonObject
        {
            ["openapi"] = "3.0.3",
            ["info"] = info,
            ["paths"] = new JsonObject(),
        };
        if (swagger.Servers.Count > 0)
            spec["servers"] = swagger.Servers.DeepClone();
        if (swagger.AuthSchemes.Count > 0)
        {
            spec["components"] = new JsonObject
            {
                ["securitySchemes"] = swagger.AuthSchemes.DeepClone(),
            };
        }
        if (swagger.AuthGlobal.Count > 0)
            spec["security"] = swagger.AuthGlobal.DeepClone();

        FillPaths((JsonObject)spec["paths"]!, version);
        return spec;
    }

    static void FillPaths(JsonObject paths, string? versionFilter)
    {
        foreach (var entry in Route.Snapshot())
        {
            if (!MatchesVersion(entry.Version, versionFilter)) continue;

            foreach (var slot in entry.Slots)
            {
                var path = slot.Path.StartsWith('/') ? slot.Path : "/" + slot.Path;
                if (paths[path] is not JsonObject methods)
                {
                    methods = new JsonObject();
                    paths[path] = methods;
                }

                var pathParams = PathParams(path);
                var methodLower = slot.HttpMethod.ToLowerInvariant();
                var methodUpper = slot.HttpMethod.ToUpperInvariant();
                var parameters = new JsonArray();
                foreach (var name in pathParams)
                {
                    parameters.Add(new JsonObject
                    {
                        ["name"] = name,
                        ["in"] = "path",
                        ["required"] = true,
                        ["schema"] = new JsonObject { ["type"] = "string" },
                    });
                }

                var bodyProperties = new JsonObject();
                var bodyRequired = new List<string>();
                foreach (var spec in slot.Specs)
                {
                    var nullable = spec.Optional || spec.HasDefault;
                    var required = !nullable;
                    if (pathParams.Contains(spec.Name, StringComparer.Ordinal))
                    {
                        parameters.Add(new JsonObject
                        {
                            ["name"] = spec.Name,
                            ["in"] = "path",
                            ["required"] = required,
                            ["schema"] = HandlerBinding.OpenApiSchema(spec.Kind, nullable),
                        });
                    }
                    else if (methodUpper is "POST" or "PUT" or "PATCH")
                    {
                        bodyProperties[spec.Name] = HandlerBinding.OpenApiSchema(spec.Kind, nullable);
                        if (required)
                            bodyRequired.Add(spec.Name);
                    }
                    else
                    {
                        parameters.Add(new JsonObject
                        {
                            ["name"] = spec.Name,
                            ["in"] = "query",
                            ["required"] = required,
                            ["schema"] = HandlerBinding.OpenApiSchema(spec.Kind, nullable),
                        });
                    }
                }

                var tags = new JsonArray();
                foreach (var tag in slot.Tags)
                    tags.Add(tag);

                var operation = new JsonObject
                {
                    ["tags"] = tags,
                    ["summary"] = slot.Title ?? $"{entry.ApiClass.Name}.{slot.Handler.Name}",
                    ["description"] = slot.Desc ?? "",
                    ["deprecated"] = slot.Deprecated,
                    ["operationId"] = $"{entry.ApiClass.Name}_{slot.Handler.Name}",
                    ["responses"] = new JsonObject
                    {
                        ["200"] = new JsonObject
                        {
                            ["description"] = "OK",
                            ["content"] = new JsonObject
                            {
                                ["application/json"] = new JsonObject
                                {
                                    ["schema"] = new JsonObject { ["type"] = "object" },
                                },
                            },
                        },
                    },
                };

                if (parameters.Count > 0)
                    operation["parameters"] = parameters;

                if (bodyProperties.Count > 0)
                {
                    var requiredArray = new JsonArray();
                    foreach (var name in bodyRequired)
                        requiredArray.Add(name);

                    operation["requestBody"] = new JsonObject
                    {
                        ["required"] = bodyRequired.Count > 0,
                        ["content"] = new JsonObject
                        {
                            ["application/json"] = new JsonObject
                            {
                                ["schema"] = new JsonObject
                                {
                                    ["type"] = "object",
                                    ["properties"] = bodyProperties,
                                    ["required"] = requiredArray,
                                },
                            },
                        },
                    };
                }

                methods[methodLower] = operation;
            }
        }
    }

    static bool MatchesVersion(string? routeVersion, string? filter)
    {
        var version = NormalizeVersion(routeVersion);
        if (filter is null) return true;
        var label = NormalizeVersion(filter);
        if (string.IsNullOrEmpty(label) || label.Equals(UnversionedName, StringComparison.OrdinalIgnoreCase))
            return string.IsNullOrEmpty(version);
        return version.Equals(label, StringComparison.OrdinalIgnoreCase);
    }

    static string NormalizeVersion(string? value) => (value ?? "").Trim().Trim('/');

    static List<string> PathParams(string path) =>
        path.Split('/')
            .Where(seg =>
                (seg.StartsWith('{') && seg.EndsWith('}') && seg.Length > 2) ||
                (seg.StartsWith('[') && seg.EndsWith(']') && seg.Length > 2))
            .Select(seg => seg[1..^1])
            .ToList();

    static string UiHtml(SwaggerConfig swagger, string openapiUrl, string? primaryName = null)
    {
        var uiOpts = swagger.Ui.DeepClone().AsObject() ?? new JsonObject();
        uiOpts.Remove("presets");
        uiOpts.Remove("plugins");
        uiOpts.Remove("layout");

        if (swagger.Urls is { Count: > 0 })
        {
            uiOpts.Remove("url");
            uiOpts["urls"] = swagger.Urls.DeepClone();
            var name = primaryName
                ?? AsString(swagger.Urls[0]?["name"]);
            if (!string.IsNullOrEmpty(name))
                uiOpts["urls.primaryName"] = name;
        }
        else
        {
            uiOpts["url"] = openapiUrl;
            uiOpts.Remove("urls");
            uiOpts.Remove("urls.primaryName");
        }
        uiOpts["dom_id"] = "#swagger-ui";

        var uiJson = uiOpts.ToJsonString().Replace("</", "<\\/");
        var oauthJson = swagger.AuthOauth.Count > 0
            ? swagger.AuthOauth.ToJsonString().Replace("</", "<\\/")
            : "null";
        var title = JsonSerializer.Serialize(swagger.PageTitle).Trim('"');
        var navbarEnabled = swagger.NavbarEnabled;
        var hideUrlCss = navbarEnabled && !swagger.ShowUrlInput
            ? """
    <style>
      .swagger-ui .topbar .download-url-input,
      .swagger-ui .topbar .download-url-button { display: none !important; }
    </style>
"""
            : "";
        var standalone = navbarEnabled
            ? """<script src="https://unpkg.com/swagger-ui-dist/swagger-ui-standalone-preset.js"></script>"""
            : "";
        var navbarJs = navbarEnabled ? "true" : "false";

        return $$"""
<!doctype html>
<html>
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>{{title}}</title>
    <link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist/swagger-ui.css" />
    {{hideUrlCss}}
  </head>
  <body>
    <div id="swagger-ui"></div>
    <script src="https://unpkg.com/swagger-ui-dist/swagger-ui-bundle.js"></script>
    {{standalone}}
    <script>
      window.onload = function() {
        var opts = {{uiJson}};
        opts.presets = [SwaggerUIBundle.presets.apis];
        opts.plugins = [SwaggerUIBundle.plugins.DownloadUrl];
        if ({{navbarJs}} && typeof SwaggerUIStandalonePreset !== 'undefined') {
          opts.presets.push(SwaggerUIStandalonePreset);
          opts.layout = 'StandaloneLayout';
        } else {
          opts.layout = 'BaseLayout';
        }
        var ui = SwaggerUIBundle(opts);
        var oauth = {{oauthJson}};
        if (oauth && typeof ui.initOAuth === 'function') {
          ui.initOAuth(oauth);
        }
        window.ui = ui;
      };
    </script>
  </body>
</html>
""";
    }

    static bool IsDisabledPath(JsonNode? value)
    {
        if (value is null || value.GetValueKind() is JsonValueKind.Null or JsonValueKind.False)
            return true;
        if (value is JsonValue v && v.TryGetValue<string>(out var s))
            return s is "" or "false" or "False" or "0" or "off" or "no";
        return false;
    }

    static bool Truthy(JsonNode? value, bool defaultValue)
    {
        if (value is null || value.GetValueKind() == JsonValueKind.Null)
            return defaultValue;
        if (value.GetValueKind() == JsonValueKind.False) return false;
        if (value.GetValueKind() == JsonValueKind.True) return true;
        if (value is JsonValue v)
        {
            if (v.TryGetValue<bool>(out var b)) return b;
            if (v.TryGetValue<int>(out var n)) return n != 0;
            if (v.TryGetValue<string>(out var s))
            {
                return s.ToLowerInvariant() switch
                {
                    "false" or "0" or "off" or "no" => false,
                    "true" or "1" or "on" or "yes" => true,
                    _ => !string.IsNullOrWhiteSpace(s),
                };
            }
        }
        return true;
    }

    static JsonObject? AsObject(JsonNode? value) =>
        value as JsonObject ?? (value is JsonValue ? null : value?.Deserialize<JsonObject>());

    static string? AsString(JsonNode? value)
    {
        if (value is null || value.GetValueKind() is JsonValueKind.Null) return null;
        if (value is JsonValue v && v.TryGetValue<string>(out var s)) return s;
        return value.ToString();
    }
}
