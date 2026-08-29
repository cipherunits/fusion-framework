// Built-in middleware + AddHeader / DeleteHeader demo.
// Place fusion.dev.json beside the host app (see examples/fusion.dev.json).

using FusionFramework;

[Route("api/[module]", Tags = new[] { "demo" })]
public class DemoModule : FusionBaseApi
{
    public object Get()
    {
        State.TryGetValue("request_id", out var rid);
        return Response(new { message = "secure defaults", request_id = rid?.ToString() }, Status.HTTP_SUCCESS);
    }

    [DeleteHeader("X-Powered-By", "X-Framework")]
    [AddHeader("Cache-Control", "public, max-age=60")]
    [AddHeader("X-Demo", "1")]
    public object Post() =>
        Response(new { message = "custom headers" }, Status.HTTP_SUCCESS);
}

// Route.Register<DemoModule>();
// SettingsStore.Current.EnsureLoaded(Directory.GetCurrentDirectory());
// var settings = SettingsStore.GetSettings();
// using var app = new FusionApp(settings);
// foreach (var mw in BuiltinMiddleware.FromSettings(settings))
//     app.Use(mw);
// app.Listen();
