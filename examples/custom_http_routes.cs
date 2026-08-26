// Copy into your C# host Program.cs (requires FusionFramework reference).
//
// Routes:
//   GET /api/user              -> Get()
//   GET /api/user/test/user    -> UserAction()   ([action] -> user)

using FusionFramework;

[Route("/api/[module]", Tags = new[] { "users" })]
public class UserModule : FusionBaseApi
{
    public object Get() =>
        Response(new { mode = "convention" }, Status.HTTP_SUCCESS);

    [HttpGet("test/[action]", Title = "User action")]
    public object UserAction() =>
        Response(new { mode = "custom", action = "user" }, Status.HTTP_SUCCESS);
}

// Program entry:
// Route.Register<UserModule>();
// SettingsStore.Current.EnsureLoaded(Directory.GetCurrentDirectory());
// using var app = new FusionApp(SettingsStore.GetSettings());
// app.Listen();
