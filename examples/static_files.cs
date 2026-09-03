// Serve images/CSS with Middleware.StaticFiles (WhiteNoise-style).
// app.Use(Middleware.StaticFiles(root: "static", prefix: "/static"));
// <img src="/static/logo.png">

using FusionFramework;

var staticDir = Path.Combine(AppContext.BaseDirectory, "static_files_assets");
Directory.CreateDirectory(staticDir);
var logo = Path.Combine(staticDir, "logo.png");
if (!File.Exists(logo))
    File.WriteAllBytes(logo, new byte[] { 0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a });

var settings = new FusionSettings();
settings.EnsureLoaded();
var app = new FusionApp(settings);
app.Use(Middleware.StaticFiles(root: staticDir, prefix: "/static"));

[Route("/api/ping")]
sealed class PingModule : FusionBaseApi
{
    public object Get() => Response(new { ok = true });
}

Route.Register<PingModule>();
app.Listen();
