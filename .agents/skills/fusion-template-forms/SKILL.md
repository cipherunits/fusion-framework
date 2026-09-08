---
name: fusion-template-forms
description: >-
  Documents FusionBaseTemplate form helpers (form, ok, fail), context vs get vs
  post, and SPA-friendly data-fusion-form + fusion/form.js. Use when building
  HTML forms that POST to the same page.
---

# Template forms (SPA-friendly)

## Mental model

| Method | Role |
|--------|------|
| `context()` | Template **data** (title, fields). Not an HTTP verb. |
| `get()` | **HTTP GET** — renders `context()` as HTML (or JSON if `Accept: application/json`). Rarely override. |
| `post()` | **HTTP POST** — read `form`, validate with normal `if`, return `ok` / `fail`. |

## API

| Python | Node | C# |
|--------|------|-----|
| `self.form` | `this.form` | `Form` |
| `self.fail(errors, message=..., **fields)` | `this.fail(errors, { message, ...fields })` | `Fail(errors, message, fields)` |
| `self.ok(message=..., **fields)` | `this.ok({ message, ...fields })` | `Ok(message, fields)` |

- JSON clients (`Accept: application/json` or SPA fetch) get `{ ok, message, errors, fields }`.
- HTML clients re-render the **same** template with errors/fields (no separate “success page” required).

## SPA markup

```html
<form method="post" action="/register" data-fusion-form>
  <div id="fusion-form-status"></div>
  <input name="phone" />
  <span class="fusion-field-error" data-field="phone"></span>
  <button type="submit">Save</button>
</form>
<script>{% include "fusion/form.js" %}</script>
```

Built-in script lives at `fusion/form.js` (embedded in fusion-core). Without JS, classic POST still works via `ok`/`fail` HTML path.

## Example

```python
def post(self):
    form = self.form
    if not form.get("phone"):
        return self.fail({"phone": "required"}, **form)
    return self.ok(message="Saved.", **form)
```

See `examples/template_form.{py,mjs,cs}`.
