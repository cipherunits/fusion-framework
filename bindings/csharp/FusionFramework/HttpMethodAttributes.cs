namespace FusionFramework;

/// <summary>Base attribute for custom HTTP method routes on <see cref="FusionBaseApi"/> handlers.</summary>
[AttributeUsage(AttributeTargets.Method, AllowMultiple = false, Inherited = false)]
public abstract class HttpMethodAttribute : Attribute
{
    public string Method { get; }
    public string Route { get; }
    public string[]? Tags { get; set; }
    public string? Desc { get; set; }
    public string? Title { get; set; }
    public bool Deprecated { get; set; }

    protected HttpMethodAttribute(string method, string route)
    {
        Method = method;
        Route = route;
    }
}

[AttributeUsage(AttributeTargets.Method, AllowMultiple = false, Inherited = false)]
public sealed class HttpGetAttribute(string route) : HttpMethodAttribute("get", route);

[AttributeUsage(AttributeTargets.Method, AllowMultiple = false, Inherited = false)]
public sealed class HttpPostAttribute(string route) : HttpMethodAttribute("post", route);

[AttributeUsage(AttributeTargets.Method, AllowMultiple = false, Inherited = false)]
public sealed class HttpPutAttribute(string route) : HttpMethodAttribute("put", route);

[AttributeUsage(AttributeTargets.Method, AllowMultiple = false, Inherited = false)]
public sealed class HttpPatchAttribute(string route) : HttpMethodAttribute("patch", route);

[AttributeUsage(AttributeTargets.Method, AllowMultiple = false, Inherited = false)]
public sealed class HttpDeleteAttribute(string route) : HttpMethodAttribute("delete", route);

[AttributeUsage(AttributeTargets.Method, AllowMultiple = false, Inherited = false)]
public sealed class HttpHeadAttribute(string route) : HttpMethodAttribute("head", route);

[AttributeUsage(AttributeTargets.Method, AllowMultiple = false, Inherited = false)]
public sealed class HttpOptionsAttribute(string route) : HttpMethodAttribute("options", route);
