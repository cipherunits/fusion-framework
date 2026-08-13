using System.Reflection;
using System.Runtime.InteropServices;
using System.Text;

namespace FusionFramework;

internal static class Native
{
    const string LibName = "fusion_ffi";

    static Native()
    {
        NativeLibrary.SetDllImportResolver(typeof(Native).Assembly, Resolve);
    }

    static IntPtr Resolve(string libraryName, Assembly assembly, DllImportSearchPath? searchPath)
    {
        if (!libraryName.Contains("fusion_ffi", StringComparison.OrdinalIgnoreCase))
            return IntPtr.Zero;

        var candidates = new List<string>();
        var env = Environment.GetEnvironmentVariable("FUSION_FFI_PATH");
        if (!string.IsNullOrWhiteSpace(env))
            candidates.Add(env);

        var baseDir = AppContext.BaseDirectory;
        candidates.Add(Path.Combine(baseDir, LibFile()));
        candidates.Add(Path.Combine(baseDir, "runtimes", Rid(), "native", LibFile()));

        // Monorepo: .../bindings/csharp/FusionFramework/bin/Debug/net8.0 → repo root
        var dir = new DirectoryInfo(baseDir);
        for (var i = 0; i < 8 && dir != null; i++)
        {
            foreach (var profile in new[] { "debug", "release" })
            {
                candidates.Add(Path.Combine(dir.FullName, "target", profile, LibFile()));
            }
            dir = dir.Parent;
        }
        foreach (var profile in new[] { "debug", "release" })
        {
            candidates.Add(Path.Combine(Directory.GetCurrentDirectory(), "target", profile, LibFile()));
        }

        foreach (var path in candidates.Distinct())
        {
            if (File.Exists(path) && NativeLibrary.TryLoad(path, out var handle))
                return handle;
        }

        if (NativeLibrary.TryLoad(LibName, assembly, searchPath, out var fallback))
            return fallback;

        throw new DllNotFoundException(
            $"Could not load {LibFile()}. Build with `cargo build -p fusion-ffi` " +
            "or set FUSION_FFI_PATH to the shared library.");
    }

    static string LibFile() =>
        OperatingSystem.IsWindows() ? "fusion_ffi.dll" :
        OperatingSystem.IsMacOS() ? "libfusion_ffi.dylib" :
        "libfusion_ffi.so";

    static string Rid() =>
        OperatingSystem.IsWindows() ? "win-x64" :
        OperatingSystem.IsMacOS() ? (RuntimeInformation.OSArchitecture == Architecture.Arm64 ? "osx-arm64" : "osx-x64") :
        (RuntimeInformation.OSArchitecture == Architecture.Arm64 ? "linux-arm64" : "linux-x64");

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr fusion_string_dup([MarshalAs(UnmanagedType.LPUTF8Str)] string s);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    public static extern void fusion_string_free(IntPtr s);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr fusion_app_new();

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    public static extern void fusion_app_free(IntPtr app);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    public static extern int fusion_app_set_settings(IntPtr app, IntPtr settings);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    public static extern int fusion_app_route(
        IntPtr app,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string method,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string path,
        FusionHandlerFn handler,
        IntPtr userData);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    public static extern int fusion_app_listen(
        IntPtr app,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string? host,
        ushort port);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    public static extern int fusion_app_listen_from_settings(IntPtr app);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr fusion_settings_new();

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    public static extern void fusion_settings_free(IntPtr settings);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    public static extern int fusion_settings_ensure_loaded(
        IntPtr settings,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string? extraRootsJson);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    public static extern int fusion_settings_load_json(
        IntPtr settings,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string? path,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string? env,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string? extraRootsJson);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    public static extern int fusion_settings_merge(
        IntPtr settings,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string jsonObject);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr fusion_settings_get(
        IntPtr settings,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string key,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string? defaultJson);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr fusion_settings_host(IntPtr settings);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    public static extern ushort fusion_settings_port(IntPtr settings);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    public static extern int fusion_settings_debug(IntPtr settings);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr fusion_settings_env(IntPtr settings);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr fusion_resolve_route_path(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string template,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string className);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr fusion_api_resource_name(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string className);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr fusion_http_methods();

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr fusion_http_status_codes();

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr fusion_http_header_constants();

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr fusion_header_attachment(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string filename);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr fusion_header_inline(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string? filename);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr fusion_header_content_type(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string mediaType,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string? charset);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr fusion_header_location(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string url);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr fusion_header_cache_control(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string value);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr fusion_header_download(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string filename,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string? mediaType);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr fusion_fingerprint_headers();

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    public delegate IntPtr FusionHandlerFn(
        IntPtr userData,
        IntPtr method,
        IntPtr path,
        IntPtr headersJson,
        IntPtr body,
        IntPtr paramsJson,
        IntPtr queryJson,
        IntPtr stateJson);

    public static string? PtrToUtf8(IntPtr p)
    {
        if (p == IntPtr.Zero) return null;
        return Marshal.PtrToStringUTF8(p);
    }

    public static string TakeUtf8(IntPtr p)
    {
        try
        {
            return PtrToUtf8(p) ?? "";
        }
        finally
        {
            if (p != IntPtr.Zero)
                fusion_string_free(p);
        }
    }
}
