using System.Text.Json;

namespace FusionFramework;

/// <summary>
/// HTTP header name constants and helpers (mirrors fusion-core).
/// Helpers return dictionaries suitable for <see cref="FusionBaseApi.Response"/>.
/// </summary>
public static class Header
{
    public static readonly IReadOnlyDictionary<string, string> Constants;

    public const string ContentType = "Content-Type";
    public const string ContentDisposition = "Content-Disposition";
    public const string Location = "Location";
    public const string Authorization = "Authorization";
    public const string CacheControl = "Cache-Control";
    public const string ApplicationJson = "application/json";
    public const string ApplicationOctetStream = "application/octet-stream";
    public const string ApplicationPdf = "application/pdf";
    public const string TextCsv = "text/csv";
    public const string TextPlainUtf8 = "text/plain; charset=utf-8";

    // Pascal aliases matching Python/Node CONST_NAME style via indexer / Constants map.
    public static string CONTENT_TYPE => Get("CONTENT_TYPE", ContentType);
    public static string CONTENT_DISPOSITION => Get("CONTENT_DISPOSITION", ContentDisposition);
    public static string LOCATION => Get("LOCATION", Location);
    public static string AUTHORIZATION => Get("AUTHORIZATION", Authorization);
    public static string APPLICATION_JSON => Get("APPLICATION_JSON", ApplicationJson);
    public static string APPLICATION_OCTET_STREAM => Get("APPLICATION_OCTET_STREAM", ApplicationOctetStream);
    public static string APPLICATION_PDF => Get("APPLICATION_PDF", ApplicationPdf);
    public static string TEXT_CSV => Get("TEXT_CSV", TextCsv);

    static Header()
    {
        var map = new Dictionary<string, string>(StringComparer.Ordinal);
        try
        {
            var json = Native.TakeUtf8(Native.fusion_http_header_constants());
            using var doc = JsonDocument.Parse(json);
            foreach (var prop in doc.RootElement.EnumerateObject())
                map[prop.Name] = prop.Value.GetString() ?? "";
        }
        catch
        {
            map["CONTENT_TYPE"] = ContentType;
            map["CONTENT_DISPOSITION"] = ContentDisposition;
            map["LOCATION"] = Location;
            map["AUTHORIZATION"] = Authorization;
            map["APPLICATION_JSON"] = ApplicationJson;
            map["APPLICATION_OCTET_STREAM"] = ApplicationOctetStream;
            map["APPLICATION_PDF"] = ApplicationPdf;
            map["TEXT_CSV"] = TextCsv;
        }
        Constants = map;
    }

    public static string Get(string name, string fallback = "") =>
        Constants.TryGetValue(name, out var v) ? v : fallback;

    public static Dictionary<string, string> Attachment(string filename) =>
        ParseMap(Native.TakeUtf8(Native.fusion_header_attachment(filename)));

    public static Dictionary<string, string> Inline(string? filename = null) =>
        ParseMap(Native.TakeUtf8(Native.fusion_header_inline(filename ?? "")));

    public static Dictionary<string, string> ContentTypeValue(string mediaType, string? charset = null) =>
        ParseMap(Native.TakeUtf8(Native.fusion_header_content_type(mediaType, charset ?? "")));

    public static Dictionary<string, string> LocationHeader(string url) =>
        ParseMap(Native.TakeUtf8(Native.fusion_header_location(url)));

    public static Dictionary<string, string> CacheControlValue(string value) =>
        ParseMap(Native.TakeUtf8(Native.fusion_header_cache_control(value)));

    /// <summary>Content-Type + Content-Disposition attachment for downloads.</summary>
    public static Dictionary<string, string> Download(string filename, string? mediaType = null) =>
        ParseMap(Native.TakeUtf8(Native.fusion_header_download(filename, mediaType ?? "")));

    /// <summary>Default Fusion identity headers (X-Powered-By, X-Framework, X-Fusion-Version).</summary>
    public static Dictionary<string, string> Fingerprint()
    {
        try
        {
            return ParseMap(Native.TakeUtf8(Native.fusion_fingerprint_headers()));
        }
        catch
        {
            return new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase)
            {
                ["X-Powered-By"] = "Fusion Framework",
                ["X-Framework"] = "Fusion",
                ["X-Fusion-Version"] = "1.3.0",
            };
        }
    }
    static Dictionary<string, string> ParseMap(string json)
    {
        var map = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
        if (string.IsNullOrWhiteSpace(json)) return map;
        using var doc = JsonDocument.Parse(json);
        foreach (var prop in doc.RootElement.EnumerateObject())
            map[prop.Name] = prop.Value.GetString() ?? prop.Value.ToString();
        return map;
    }
}
