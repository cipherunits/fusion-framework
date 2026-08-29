using System.Reflection;
using System.Text.Json.Nodes;

namespace FusionFramework;

/// <summary>Merge headers into a handler response (method-level).</summary>
[AttributeUsage(AttributeTargets.Method, AllowMultiple = true, Inherited = false)]
public sealed class AddHeaderAttribute : Attribute
{
    public string Name { get; }
    public string Value { get; }

    public AddHeaderAttribute(string name, string value)
    {
        Name = name;
        Value = value;
    }
}

/// <summary>Strip headers from a handler response and suppress wire fingerprint re-add.</summary>
[AttributeUsage(AttributeTargets.Method, AllowMultiple = true, Inherited = false)]
public sealed class DeleteHeaderAttribute : Attribute
{
    public string[] Names { get; }

    public DeleteHeaderAttribute(params string[] names) => Names = names;
}

internal static class HeaderOps
{
    public static object? ApplyMethodHeaderOps(MethodInfo method, object? result)
    {
        var addAttrs = method.GetCustomAttributes<AddHeaderAttribute>(false).ToList();
        var deleteAttrs = method.GetCustomAttributes<DeleteHeaderAttribute>(false).ToList();
        if (addAttrs.Count == 0 && deleteAttrs.Count == 0)
            return result;

        var envelope = EnsureEnvelope(result);
        var headers = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
        if (envelope.TryGetValue("headers", out var existing) && existing is IDictionary<string, string> map)
        {
            foreach (var kv in map) headers[kv.Key] = kv.Value;
        }

        foreach (var attr in addAttrs)
            headers[attr.Name] = attr.Value;

        var suppress = new List<string>();
        if (envelope.TryGetValue("suppress_headers", out var rawSuppress) && rawSuppress is IEnumerable<string> list)
            suppress.AddRange(list);

        foreach (var attr in deleteAttrs)
        {
            foreach (var name in attr.Names)
            {
                headers.Remove(name);
                if (!suppress.Any(s => s.Equals(name, StringComparison.OrdinalIgnoreCase)))
                    suppress.Add(name);
            }
        }

        envelope["headers"] = headers;
        if (suppress.Count > 0)
            envelope["suppress_headers"] = suppress;
        return envelope;
    }

    static Dictionary<string, object?> EnsureEnvelope(object? result)
    {
        if (result is Dictionary<string, object?> dict && dict.ContainsKey("status"))
            return dict;
        return new Dictionary<string, object?>
        {
            ["status"] = 200,
            ["body"] = result,
        };
    }
}
