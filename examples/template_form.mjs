/**
 * SPA-friendly template form (form / ok / fail).
 *
 *   node examples/template_form.mjs
 */
import { createRequire } from 'node:module'
import { mkdirSync, writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const require = createRequire(import.meta.url)
const {
  FusionApp,
  FusionBaseTemplate,
  route,
  settings,
} = require('../crates/fusion-node')

const demoDir = dirname(fileURLToPath(import.meta.url))
const tplRoot = join(demoDir, 'templates_form')
mkdirSync(tplRoot, { recursive: true })
writeFileSync(
  join(tplRoot, 'register.html'),
  `<!DOCTYPE html>
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
`,
  )

settings.merge({ templates: { dir: tplRoot } })

class RegisterPage extends FusionBaseTemplate {
  static template = 'register.html'

  context() {
    return {
      title: 'Register',
      message: 'Fill the form.',
      ok: false,
      errors: {},
      name: '',
      phone: '',
    }
  }

  post() {
    const form = this.form
    const errors = {}
    if (!form.phone) errors.phone = 'phone is required'
    if (!form.name) errors.name = 'name is required'
    const safe = { name: form.name || '', phone: form.phone || '' }
    if (Object.keys(errors).length) {
      return this.fail(errors, { message: 'Fix the errors.', ...safe })
    }
    console.log('submitted', safe)
    return this.ok({ message: 'Saved.', ...safe })
  }
}

route('/register')(RegisterPage)

const app = new FusionApp(settings)
await app.listen()
