using System.Reflection;
using System.Text.Json;
using System.Text.Json.Nodes;

namespace FusionFramework;

internal enum HandlerParamKind
{
    String,
    Int,
    Float,
    Bool,
    Auto,
}

internal sealed class HandlerParamSpec
{
    public required string Name { get; init; }
    public HandlerParamKind Kind { get; init; }
    public bool Optional { get; init; }
    public bool HasDefault { get; init; }
    public object? DefaultValue { get; init; }
    public required Type ParameterType { get; init; }
}

/// <summary>Bind handler parameters from path/query/body (fusion-core dispatch parity).</summary>
internal static class HandlerBinding
{
    static readonly string[] BodyMethods = ["POST", "PUT", "PATCH"];

    public static List<HandlerParamSpec> ExtractSpecs(MethodInfo method)
    {
        var specs = new List<HandlerParamSpec>();
        foreach (var p in method.GetParameters())
        {
            if (p.ParameterType == typeof(CancellationToken))
                continue;

            var (kind, optional, underlying) = ClassifyType(p.ParameterType);
            specs.Add(new HandlerParamSpec
            {
                Name = p.Name ?? "",
                Kind = kind,
                Optional = optional || p.HasDefaultValue,
                HasDefault = p.HasDefaultValue,
                DefaultValue = p.HasDefaultValue ? p.DefaultValue : null,
                ParameterType = underlying ?? p.ParameterType,
            });
        }
        return specs;
    }

    public static object?[] BindArgs(MethodInfo method, FusionRequest req, IReadOnlyList<HandlerParamSpec> specs)
    {
        var parameters = method.GetParameters();
        if (parameters.Length == 0)
            return Array.Empty<object?>();

        var specByName = specs.ToDictionary(s => s.Name, StringComparer.Ordinal);
        var httpMethod = (req.Method ?? "").ToUpperInvariant();
        var bodyFields = BodyMethods.Contains(httpMethod)
            ? ParseJsonObject(req.Body)
            : null;

        var args = new object?[parameters.Length];
        for (var i = 0; i < parameters.Length; i++)
        {
            var p = parameters[i];
            if (p.ParameterType == typeof(CancellationToken))
            {
                args[i] = CancellationToken.None;
                continue;
            }

            if (!specByName.TryGetValue(p.Name ?? "", out var spec))
            {
                args[i] = p.HasDefaultValue ? p.DefaultValue : DefaultFor(p.ParameterType);
                continue;
            }

            string? source = null;
            JsonNode? raw = null;

            if (req.Params.TryGetValue(spec.Name, out var pathVal))
            {
                source = "path";
                raw = JsonValue.Create(pathVal);
            }
            else if (bodyFields is not null && bodyFields.TryGetValue(spec.Name, out var bodyVal))
            {
                source = "body";
                raw = bodyVal;
            }
            else if (req.Query.TryGetValue(spec.Name, out var queryVal))
            {
                source = "query";
                raw = JsonValue.Create(queryVal);
            }

            if (source is null)
            {
                if (spec.HasDefault)
                {
                    args[i] = spec.DefaultValue;
                    continue;
                }
                args[i] = spec.Optional ? null : DefaultFor(p.ParameterType);
                continue;
            }

            args[i] = CoerceToType(raw, p.ParameterType, spec);
        }

        return args;
    }

    public static JsonObject OpenApiSchema(HandlerParamKind kind, bool nullable)
    {
        JsonObject schema = kind switch
        {
            HandlerParamKind.Int => new JsonObject { ["type"] = "integer", ["format"] = "int64" },
            HandlerParamKind.Float => new JsonObject { ["type"] = "number" },
            HandlerParamKind.Bool => new JsonObject { ["type"] = "boolean" },
            _ => new JsonObject { ["type"] = "string" },
        };
        if (nullable)
            schema["nullable"] = true;
        return schema;
    }

    static (HandlerParamKind Kind, bool Optional, Type? Underlying) ClassifyType(Type type)
    {
        var underlying = Nullable.GetUnderlyingType(type);
        if (underlying is not null)
            return (MapKind(underlying), true, underlying);

        return (MapKind(type), false, type);
    }

    static HandlerParamKind MapKind(Type type)
    {
        if (type == typeof(int) || type == typeof(long) || type == typeof(short) ||
            type == typeof(byte) || type == typeof(uint) || type == typeof(ulong) ||
            type == typeof(ushort) || type == typeof(sbyte))
            return HandlerParamKind.Int;
        if (type == typeof(float) || type == typeof(double) || type == typeof(decimal))
            return HandlerParamKind.Float;
        if (type == typeof(bool))
            return HandlerParamKind.Bool;
        if (type == typeof(string))
            return HandlerParamKind.String;
        return HandlerParamKind.Auto;
    }

    static Dictionary<string, JsonNode?>? ParseJsonObject(string? body)
    {
        if (string.IsNullOrWhiteSpace(body))
            return null;
        try
        {
            var node = JsonNode.Parse(body);
            if (node is not JsonObject obj)
                return null;
            return obj.ToDictionary(kv => kv.Key, kv => kv.Value);
        }
        catch
        {
            return null;
        }
    }

    static object? CoerceToType(JsonNode? raw, Type targetType, HandlerParamSpec spec)
    {
        var underlying = Nullable.GetUnderlyingType(targetType) ?? targetType;
        if (raw is null || raw.GetValueKind() == JsonValueKind.Null)
            return spec.Optional ? null : DefaultFor(targetType);

        if (underlying == typeof(string))
            return raw.GetValueKind() == JsonValueKind.String ? raw.GetValue<string>() : raw.ToJsonString();

        if (underlying == typeof(bool))
        {
            if (raw is JsonValue v && v.TryGetValue<bool>(out var b)) return b;
            var s = raw.ToString().Trim().ToLowerInvariant();
            return s is "1" or "true" or "yes" or "on";
        }

        if (underlying == typeof(int))
        {
            if (raw is JsonValue v && v.TryGetValue<int>(out var parsed)) return parsed;
            return int.TryParse(raw.ToString(), out var parsedInt) ? parsedInt : 0;
        }

        if (underlying == typeof(long))
        {
            if (raw is JsonValue v && v.TryGetValue<long>(out var parsed)) return parsed;
            return long.TryParse(raw.ToString(), out var parsedLong) ? parsedLong : 0L;
        }

        if (underlying == typeof(float))
        {
            if (raw is JsonValue v && v.TryGetValue<float>(out var parsed)) return parsed;
            return float.TryParse(raw.ToString(), out var parsedFloat) ? parsedFloat : 0f;
        }

        if (underlying == typeof(double))
        {
            if (raw is JsonValue v && v.TryGetValue<double>(out var parsed)) return parsed;
            return double.TryParse(raw.ToString(), out var parsedDouble) ? parsedDouble : 0d;
        }

        if (underlying == typeof(decimal))
        {
            if (raw is JsonValue v && v.TryGetValue<decimal>(out var parsed)) return parsed;
            return decimal.TryParse(raw.ToString(), out var parsedDecimal) ? parsedDecimal : 0m;
        }

        return raw.Deserialize(targetType);
    }

    static object? DefaultFor(Type type) =>
        type.IsValueType ? Activator.CreateInstance(type) : null;
}
