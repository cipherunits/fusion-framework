"""SPA-friendly template form (form / ok / fail).

    python examples/template_form.py

Open http://127.0.0.1:8080/register
"""

from __future__ import annotations

from pathlib import Path

from fusion_framework import settings
from fusion_framework.app import FusionApp
from fusion_framework.route import route
from fusion_framework.template import FusionBaseTemplate

DEMO = Path(__file__).resolve().parent
settings.configure(templates={"dir": str(DEMO / "templates_form")})


@route("/register")
class RegisterPage(FusionBaseTemplate):
    template = "register.html"

    def context(self):
        return {
            "title": "Register",
            "message": "Fill the form.",
            "ok": False,
            "errors": {},
            "name": "",
            "phone": "",
        }

    def post(self):
        form = self.form
        errors = {}
        if not form.get("phone"):
            errors["phone"] = "phone is required"
        if not form.get("name"):
            errors["name"] = "name is required"
        safe = {"name": form.get("name", ""), "phone": form.get("phone", "")}
        if errors:
            return self.fail(errors, message="Fix the errors.", **safe)
        print("submitted", safe)
        return self.ok(message="Saved.", **safe)


def main() -> None:
    root = DEMO / "templates_form"
    root.mkdir(exist_ok=True)
    (root / "register.html").write_text(
        """<!DOCTYPE html>
<html>
<head><meta charset="utf-8" /><title>{{ title }}</title>
<style>
  .fusion-field-error { color: #b91c1c; }
  #fusion-form-status.fusion-form-ok { color: #15803d; }
  #fusion-form-status.fusion-form-fail { color: #b91c1c; }
</style>
</head>
<body>
  <h1>{{ title }}</h1>
  <p>{{ message }}</p>
  <div id="fusion-form-status"></div>
  <form method="post" action="/register" data-fusion-form>
    <label>Name <input name="name" value="{{ name }}" />
      <span class="fusion-field-error" data-field="name"></span></label>
    <label>Phone <input name="phone" value="{{ phone }}" />
      <span class="fusion-field-error" data-field="phone"></span></label>
    <button type="submit">Save</button>
  </form>
  <script>{% include "fusion/form.js" %}</script>
</body>
</html>
""",
        encoding="utf-8",
    )
    FusionApp(settings).listen()


if __name__ == "__main__":
    main()
