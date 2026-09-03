using System.Text.Json.Nodes;
using FusionFramework;
using Xunit;

namespace FusionFramework.Tests;

public class MiddlewareTests
{
    static object Handler(FusionRequest request) =>
        new Dictionary<string, object?>
        {
            ["status"] = 200,
            ["body"] = new Dictionary<string, object?> { ["state"] = request.State },
        };

    [Fact]
    public void BearerJwt_populates_state()
    {
        const string token =
            "eyJhbGciOiJub25lIiwidHlwIjoiSldUIn0.eyJzdWIiOiIxIiwicm9sZXMiOlsiYWRtaW4iXX0.";
        var request = new FusionRequest
        {
            Headers = new Dictionary<string, string> { ["Authorization"] = $"Bearer {token}" },
        };

        var result = Middleware.RunChain(
            request,
            new[] { Middleware.BearerJwt() },
            Handler) as Dictionary<string, object?>;

        Assert.NotNull(result);
        Assert.Equal(200, result["status"]);
        var body = Assert.IsType<Dictionary<string, object?>>(result["body"]);
        var state = Assert.IsType<Dictionary<string, JsonNode?>>(body["state"]);
        Assert.Equal("1", state["jwt"]!["sub"]!.GetValue<string>());
    }

    [Fact]
    public void RequireRoles_blocks_missing_role()
    {
        var request = new FusionRequest
        {
            State = new Dictionary<string, JsonNode?>
            {
                ["jwt"] = System.Text.Json.Nodes.JsonNode.Parse("{\"roles\":[\"user\"]}"),
            },
        };

        var result = Middleware.RunChain(
            request,
            new[] { Middleware.RequireRoles("admin") },
            Handler) as Dictionary<string, object?>;

        Assert.NotNull(result);
        Assert.Equal(403, result["status"]);
    }

    [Fact]
    public void Cors_answers_options_with_204()
    {
        var request = new FusionRequest
        {
            Method = "OPTIONS",
            Path = "/api",
            Headers = new Dictionary<string, string> { ["Origin"] = "https://example.com" },
        };

        var result = Middleware.RunChain(request, new[] { Middleware.Cors() }, Handler)
            as Dictionary<string, object?>;

        Assert.NotNull(result);
        Assert.Equal(204, result["status"]);
        var headers = Assert.IsType<Dictionary<string, string>>(result["headers"]);
        Assert.False(string.IsNullOrEmpty(headers["Access-Control-Allow-Origin"]));
    }

    [Fact]
    public void StaticFiles_serves_asset_under_prefix()
    {
        var dir = Directory.CreateTempSubdirectory("fusion-static-");
        try
        {
            var file = Path.Combine(dir.FullName, "logo.png");
            File.WriteAllBytes(file, new byte[] { 0x89, 0x50, 0x4e, 0x47 });

            var request = new FusionRequest
            {
                Method = "GET",
                Path = "/static/logo.png",
                Headers = new Dictionary<string, string>(),
            };

            var result = Middleware.RunChain(
                request,
                new[] { Middleware.StaticFiles(root: dir.FullName, prefix: "/static", maxAge: 60) },
                Handler) as Dictionary<string, object?>;

            Assert.NotNull(result);
            Assert.Equal(200, result["status"]);
            Assert.IsType<byte[]>(result["body"]);
            var headers = Assert.IsType<Dictionary<string, string>>(result["headers"]);
            Assert.Equal("image/png", headers["content-type"]);
        }
        finally
        {
            dir.Delete(recursive: true);
        }
    }
}
