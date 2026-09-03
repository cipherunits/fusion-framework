namespace FusionFramework;

/// <summary>Convenience aliases for built-in middleware factories used in scaffolded apps.</summary>
public static class BuiltinMiddleware
{
    public static FusionMiddleware FrameworkHeaders() => Middleware.FrameworkHeaders();
    public static FusionMiddleware SecurityHeaders() => Middleware.SecurityHeaders();
    public static FusionMiddleware Cors() => Middleware.Cors();
    public static FusionMiddleware CacheHeaders() => Middleware.CacheHeaders();
    public static FusionMiddleware RequestId() => Middleware.RequestId();
    public static FusionMiddleware StaticFiles(
        string root = "static",
        string prefix = "/static",
        int? maxAge = 3600,
        bool? fallthrough = null) =>
        Middleware.StaticFiles(root, prefix, maxAge, fallthrough);
}
