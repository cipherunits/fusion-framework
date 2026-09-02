using System.Reflection;
using System.Runtime.InteropServices;
using System.Text.Json;

namespace FusionFramework;

/// <summary>Managed Fusion application — mounts registered routes and listens via fusion-ffi.</summary>
public sealed class FusionApp : IDisposable
{
    readonly List<FusionMiddleware> _middleware = new();
    readonly List<GCHandle> _pins = new();
    IntPtr _app = IntPtr.Zero;
    bool _disposed;

    public FusionApp(FusionSettings? settings = null)
    {
        _app = Native.fusion_app_new();
        if (_app == IntPtr.Zero)
            throw new InvalidOperationException("fusion_app_new failed");

        settings ??= SettingsStore.Current;
        Native.fusion_app_set_settings(_app, settings.Handle);
    }

    public FusionApp Use(FusionMiddleware middleware)
    {
        _middleware.Add(middleware);
        return this;
    }

    /// <summary>Wire all registered <see cref="Route"/> entries into the native engine.</summary>
    public void Mount()
    {
        Middleware.SetActiveGlobal(_middleware);

        foreach (var entry in Route.Snapshot())
        {
            foreach (var mountSlot in entry.Slots)
            {
                var slot = new RouteSlot
                {
                    Entry = entry,
                    Path = mountSlot.Path,
                    Method = mountSlot.HttpMethod.ToUpperInvariant(),
                    Handler = mountSlot.Handler,
                    Specs = mountSlot.Specs,
                    Global = _middleware.ToList(),
                };

                Native.FusionHandlerFn cb = (userData, method, path, headersJson, body, paramsJson, queryJson, stateJson) =>
                {
                    try
                    {
                        var handle = GCHandle.FromIntPtr(userData);
                        var s = (RouteSlot)handle.Target!;
                        var req = new FusionRequest
                        {
                            Method = Native.PtrToUtf8(method) ?? s.Method,
                            Path = Native.PtrToUtf8(path) ?? "",
                            Body = Native.PtrToUtf8(body) ?? "",
                            Headers = JsonUtil.ParseStringMap(Native.PtrToUtf8(headersJson)),
                            Params = JsonUtil.ParseStringMap(Native.PtrToUtf8(paramsJson)),
                            Query = JsonUtil.ParseStringMap(Native.PtrToUtf8(queryJson)),
                            State = JsonUtil.ParseState(Native.PtrToUtf8(stateJson)),
                        };

                        var chain = s.Global.Concat(s.Entry.Middleware).ToList();
                        object? result;
                        try
                        {
                            result = Middleware.RunChain(req, chain, r =>
                            {
                                var instance = (FusionBaseApi)Activator.CreateInstance(s.Entry.ApiClass)!;
                                instance.Request = r;
                                var args = HandlerBinding.BindArgs(s.Handler, r, s.Specs);
                                // Async methods return Task/ValueTask — RunChain unwraps them.
                                return s.Handler.Invoke(instance, args);
                            });
                        }
                        catch (TargetInvocationException tie) when (tie.InnerException is HttpException hex)
                        {
                            result = hex.ToResponse();
                        }
                        catch (HttpException hex)
                        {
                            result = hex.ToResponse();
                        }

                        // Final guard: never serialize a raw Task as the response body.
                        result = Middleware.ResolveAwaitable(result);
                        var json = JsonUtil.SerializeResponse(result);
                        return Native.fusion_string_dup(json);
                    }
                    catch (Exception ex)
                    {
                        var err = JsonSerializer.Serialize(new
                        {
                            status = 500,
                            body = new { detail = ex.Message },
                        });
                        return Native.fusion_string_dup(err);
                    }
                };

                // Keep delegate + slot alive for the lifetime of the app
                slot.KeptAlive = cb;
                var gch = GCHandle.Alloc(slot);
                _pins.Add(gch);
                _pins.Add(GCHandle.Alloc(cb));

                if (Native.fusion_app_route(_app, slot.Method, slot.Path, cb, GCHandle.ToIntPtr(gch)) != 0)
                    throw new InvalidOperationException($"Failed to register {slot.Method} {slot.Path}");
            }
        }

        SwaggerDocs.Mount(this, SettingsStore.Current);
    }

    internal void AddRawRoute(string method, string path, Func<object?> handler)
    {
        Native.FusionHandlerFn cb = (_, _, _, _, _, _, _, _) =>
        {
            try
            {
                var json = JsonUtil.SerializeResponse(handler());
                return Native.fusion_string_dup(json);
            }
            catch (Exception ex)
            {
                var err = JsonSerializer.Serialize(new
                {
                    status = 500,
                    body = new { detail = ex.Message },
                });
                return Native.fusion_string_dup(err);
            }
        };
        _pins.Add(GCHandle.Alloc(cb));
        if (Native.fusion_app_route(_app, method, path, cb, IntPtr.Zero) != 0)
            throw new InvalidOperationException($"Failed to register {method} {path}");
    }

    public void Listen(string? host = null, ushort port = 0, bool? reload = null, IEnumerable<string>? watchDirs = null)
    {
        var settings = SettingsStore.Current;
        var settingsReload = Truthy(settings.Get("reload", false));
        var shouldReload = Reloader.Resolve(reload, settingsReload);

        if (shouldReload && !Reloader.IsChild)
        {
            Reloader.RunWithReloader(watchDirs);
            return;
        }

        Mount();
        host ??= settings.Host;
        if (port == 0) port = settings.Port;
        if (settings.Debug || shouldReload)
        {
            var mode = shouldReload ? " (reload)" : "";
            Console.WriteLine($"fusion listening on http://{host}:{port}{mode}");
        }

        var code = Native.fusion_app_listen(_app, host, port);
        // listen consumes the native app
        _app = IntPtr.Zero;
        if (code != 0)
            throw new InvalidOperationException("fusion_app_listen failed");
    }

    static bool Truthy(System.Text.Json.Nodes.JsonNode? node, bool fallback = false)
    {
        if (node is null) return fallback;
        if (node is System.Text.Json.Nodes.JsonValue v)
        {
            if (v.TryGetValue<bool>(out var b)) return b;
            if (v.TryGetValue<string>(out var s))
                return s is "1" or "true" or "True" or "yes" or "on";
        }
        return fallback;
    }

    public void Dispose()
    {
        if (_disposed) return;
        _disposed = true;
        if (_app != IntPtr.Zero)
        {
            Native.fusion_app_free(_app);
            _app = IntPtr.Zero;
        }
        foreach (var pin in _pins)
        {
            if (pin.IsAllocated) pin.Free();
        }
        _pins.Clear();
        GC.SuppressFinalize(this);
    }

    ~FusionApp() => Dispose();
}
