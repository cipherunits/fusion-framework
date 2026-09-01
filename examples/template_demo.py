"""Template demo — run from repo root: python examples/template_demo.py"""

from fusion_framework import settings
from fusion_framework.app import FusionApp
from fusion_framework.middleware import framework_headers
from fusion_framework.route import route
from fusion_framework.template import FusionBaseTemplate

settings.configure(**{"templates.dir": "examples/templates"})


@route("/pages/[module]")
class HomePage(FusionBaseTemplate):
    template = "home/index.html"

    def context(self):
        return {
            "title": "Fusion Templates",
            "message": "Hello from Tera!",
        }


app = FusionApp()
app.use(framework_headers())

if __name__ == "__main__":
    app.run()
