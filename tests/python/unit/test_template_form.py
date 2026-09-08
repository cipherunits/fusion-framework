"""Unit tests for template form helpers (form / ok / fail)."""

from __future__ import annotations

from fusion_framework.template import FusionBaseTemplate, parse_form_body


def test_parse_form_body_urlencoded():
    data = parse_form_body(
        "name=Ada&phone=0912",
        "application/x-www-form-urlencoded",
    )
    assert data["name"] == "Ada"
    assert data["phone"] == "0912"


def test_parse_form_body_json():
    data = parse_form_body('{"name":"Ada","phone":null}', "application/json")
    assert data["name"] == "Ada"
    assert data["phone"] == ""


class _Page(FusionBaseTemplate):
    template = "home/index.html"

    def context(self):
        return {"title": "t", "message": "m", "errors": {}, "ok": False}


def test_fail_returns_json_when_accept_json(tmp_path, monkeypatch):
    monkeypatch.setenv("FUSION_ENV", "dev")
    page = _Page(
        {
            "method": "POST",
            "path": "/register",
            "body": "phone=",
            "headers": {
                "accept": "application/json",
                "content-type": "application/x-www-form-urlencoded",
            },
            "params": {},
            "query": {},
            "state": {},
        }
    )
    page.templates_dir = str(tmp_path)
    out = page.fail({"phone": "required"}, message="bad", name="Ada")
    assert out["status"] == 400
    body = out["body"]
    assert body["ok"] is False
    assert body["errors"]["phone"] == "required"
    assert body["fields"]["name"] == "Ada"


def test_ok_returns_json_when_accept_json(tmp_path):
    page = _Page(
        {
            "method": "POST",
            "path": "/register",
            "body": "",
            "headers": {"accept": "application/json"},
            "params": {},
            "query": {},
            "state": {},
        }
    )
    page.templates_dir = str(tmp_path)
    out = page.ok(message="Saved.", name="Ada")
    assert out["status"] == 200
    assert out["body"]["ok"] is True
    assert out["body"]["message"] == "Saved."
    assert out["body"]["fields"]["name"] == "Ada"


def test_form_property_parses_body():
    page = _Page(
        {
            "method": "POST",
            "path": "/register",
            "body": "email=a%40b.com&phone=09",
            "headers": {"content-type": "application/x-www-form-urlencoded"},
            "params": {},
            "query": {},
            "state": {},
        }
    )
    assert page.form["email"] == "a@b.com"
    assert page.form["phone"] == "09"
