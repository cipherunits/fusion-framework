using System.Text.Json.Nodes;

namespace FusionFramework.Testing;

/// <summary>Helpers used by the repository test suite (not required at runtime).</summary>
public static class FusionTestSupport
{
    public static JsonObject OpenApiSpec(string? version = null) => SwaggerDocs.CreateTestSpec(version);

    public static void ClearRoutes() => Route.Clear();
}
