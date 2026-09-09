// SPA-friendly template form (Form / Ok / Fail).
// Prefer running the Python or Node example for a quick demo;
// this file shows the C# handler shape.
//
//   RegisterPage : FusionBaseTemplate
//   Context() -> title/message
//   Post() -> Form + Fail(errors) / Ok(message, fields)

using System.Text.Json.Nodes;
using FusionFramework;

[Route("/register")]
public class RegisterPage : FusionBaseTemplate
{
    static RegisterPage()
    {
        Template = "register.html";
    }

    public override Dictionary<string, JsonNode?> Context() => new()
    {
        ["title"] = "Register",
        ["message"] = "Fill the form.",
        ["ok"] = false,
        ["errors"] = new JsonObject(),
        ["name"] = "",
        ["phone"] = "",
    };

    public object Post()
    {
        var form = Form;
        var errors = new Dictionary<string, string>(StringComparer.Ordinal);
        if (string.IsNullOrWhiteSpace(form.GetValueOrDefault("phone")))
            errors["phone"] = "phone is required";
        if (string.IsNullOrWhiteSpace(form.GetValueOrDefault("name")))
            errors["name"] = "name is required";

        var safe = new Dictionary<string, string>
        {
            ["name"] = form.GetValueOrDefault("name") ?? "",
            ["phone"] = form.GetValueOrDefault("phone") ?? "",
        };

        if (errors.Count > 0)
            return Fail(errors, "Fix the errors.", safe);

        Console.WriteLine($"submitted name={safe["name"]} phone={safe["phone"]}");
        return Ok("Saved.", safe);
    }
}
