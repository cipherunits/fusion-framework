using FusionFramework;

[Route("/api/[module]/{id}")]
public class MyFirstModule : FusionBaseApi
{
    public object Get()
    {
        Params.TryGetValue("id", out var id);
        return Response($"hello from csharp, id={id}", Status.HTTP_SUCCESS);
    }
}

[Route("/")]
public class RootApi : FusionBaseApi
{
    public object Get() => Response("fusion class-based api");
}

public static class Program
{
    static readonly List<FusionMiddleware> MiddlewareList = new();

    public static void Main()
    {
        Route.Register<MyFirstModule>();
        Route.Register<RootApi>();
        SettingsStore.Current.EnsureLoaded(Directory.GetCurrentDirectory());
        using var app = new FusionApp(SettingsStore.GetSettings());
        foreach (var mw in MiddlewareList) app.Use(mw);
        app.Listen();
    }
}
