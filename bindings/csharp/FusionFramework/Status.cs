using System.Text.Json;
using System.Text.Json.Nodes;

namespace FusionFramework;

/// <summary>HTTP status constants mirrored from fusion-core (plus Codes map from FFI).</summary>
public static class Status
{
    public static readonly IReadOnlyDictionary<string, int> Codes;

    public const int HTTP_100_CONTINUE = 100;
    public const int HTTP_101_SWITCHING_PROTOCOLS = 101;
    public const int HTTP_200_OK = 200;
    public const int HTTP_SUCCESS = 200;
    public const int HTTP_201_CREATED = 201;
    public const int HTTP_202_ACCEPTED = 202;
    public const int HTTP_204_NO_CONTENT = 204;
    public const int HTTP_301_MOVED_PERMANENTLY = 301;
    public const int HTTP_302_FOUND = 302;
    public const int HTTP_304_NOT_MODIFIED = 304;
    public const int HTTP_400_BAD_REQUEST = 400;
    public const int HTTP_401_UNAUTHORIZED = 401;
    public const int HTTP_403_FORBIDDEN = 403;
    public const int HTTP_404_NOT_FOUND = 404;
    public const int HTTP_405_METHOD_NOT_ALLOWED = 405;
    public const int HTTP_409_CONFLICT = 409;
    public const int HTTP_422_UNPROCESSABLE_ENTITY = 422;
    public const int HTTP_429_TOO_MANY_REQUESTS = 429;
    public const int HTTP_500_INTERNAL_SERVER_ERROR = 500;
    public const int HTTP_502_BAD_GATEWAY = 502;
    public const int HTTP_503_SERVICE_UNAVAILABLE = 503;

    static Status()
    {
        var map = new Dictionary<string, int>(StringComparer.Ordinal);
        try
        {
            var json = Native.TakeUtf8(Native.fusion_http_status_codes());
            using var doc = JsonDocument.Parse(json);
            foreach (var prop in doc.RootElement.EnumerateObject())
            {
                if (prop.Value.TryGetInt32(out var code))
                    map[prop.Name] = code;
            }
        }
        catch
        {
            map["HTTP_SUCCESS"] = HTTP_SUCCESS;
            map["HTTP_200_OK"] = HTTP_200_OK;
            map["HTTP_201_CREATED"] = HTTP_201_CREATED;
            map["HTTP_204_NO_CONTENT"] = HTTP_204_NO_CONTENT;
            map["HTTP_401_UNAUTHORIZED"] = HTTP_401_UNAUTHORIZED;
            map["HTTP_403_FORBIDDEN"] = HTTP_403_FORBIDDEN;
            map["HTTP_404_NOT_FOUND"] = HTTP_404_NOT_FOUND;
            map["HTTP_500_INTERNAL_SERVER_ERROR"] = HTTP_500_INTERNAL_SERVER_ERROR;
        }
        Codes = map;
    }

    public static int Get(string name) =>
        Codes.TryGetValue(name, out var code) ? code : HTTP_500_INTERNAL_SERVER_ERROR;
}

/// <summary>Thrown / returned as an HTTP error envelope.</summary>
public class HttpException : Exception
{
    public int StatusCode { get; }
    public object? Detail { get; }
    public IDictionary<string, string> Headers { get; }

    public HttpException(int status, object? detail = null, IDictionary<string, string>? headers = null)
        : base(detail is string s ? s : $"HTTP {status}")
    {
        StatusCode = status;
        Detail = detail;
        Headers = headers ?? new Dictionary<string, string>();
    }

    public object ToResponse()
    {
        var body = Detail ?? Message;
        return new Dictionary<string, object?>
        {
            ["status"] = StatusCode,
            ["body"] = body is string or JsonNode
                ? body
                : JsonSerializer.SerializeToNode(body),
            ["headers"] = Headers.Count == 0
                ? null
                : Headers.ToDictionary(kv => kv.Key, kv => (object)kv.Value),
        };
    }
}
