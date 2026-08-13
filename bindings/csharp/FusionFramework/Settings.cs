using System.Text.Json;
using System.Text.Json.Nodes;

namespace FusionFramework;

/// <summary>Runtime settings loaded from fusion.&lt;env&gt;.json via FFI.</summary>
public sealed class FusionSettings : IDisposable
{
    internal IntPtr Handle { get; private set; }
    bool _disposed;

    public FusionSettings()
    {
        Handle = Native.fusion_settings_new();
        if (Handle == IntPtr.Zero)
            throw new InvalidOperationException("fusion_settings_new failed");
    }

    public void EnsureLoaded(params string[] extraRoots)
    {
        var json = extraRoots.Length == 0
            ? "[]"
            : JsonSerializer.Serialize(extraRoots);
        Native.fusion_settings_ensure_loaded(Handle, json);
    }

    public void LoadJson(string? path = null, string? env = null, params string[] extraRoots)
    {
        var roots = extraRoots.Length == 0 ? "[]" : JsonSerializer.Serialize(extraRoots);
        if (Native.fusion_settings_load_json(Handle, path, env, roots) != 0)
            throw new InvalidOperationException("settings load failed");
    }

    public void Merge(object values)
    {
        var json = values is string s ? s : JsonSerializer.Serialize(values);
        Native.fusion_settings_merge(Handle, json);
    }

    public JsonNode? Get(string key, object? defaultValue = null)
    {
        var def = defaultValue is null
            ? "null"
            : defaultValue is string ds
                ? JsonSerializer.Serialize(ds)
                : JsonSerializer.Serialize(defaultValue);
        var raw = Native.TakeUtf8(Native.fusion_settings_get(Handle, key, def));
        return JsonNode.Parse(raw);
    }

    public string Host => Native.TakeUtf8(Native.fusion_settings_host(Handle));
    public ushort Port => Native.fusion_settings_port(Handle);
    public bool Debug => Native.fusion_settings_debug(Handle) != 0;
    public string Env => Native.TakeUtf8(Native.fusion_settings_env(Handle));

    public void Dispose()
    {
        if (_disposed) return;
        _disposed = true;
        if (Handle != IntPtr.Zero)
        {
            Native.fusion_settings_free(Handle);
            Handle = IntPtr.Zero;
        }
        GC.SuppressFinalize(this);
    }

    ~FusionSettings() => Dispose();
}

public static class SettingsStore
{
    static readonly object Gate = new();
    static FusionSettings? _instance;

    public static FusionSettings Current
    {
        get
        {
            lock (Gate)
            {
                return _instance ??= new FusionSettings();
            }
        }
    }

    public static FusionSettings GetSettings() => Current;

    public static FusionSettings Configure(object values)
    {
        var s = Current;
        s.Merge(values);
        return s;
    }
}
