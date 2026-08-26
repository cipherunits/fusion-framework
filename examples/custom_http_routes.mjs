import { FusionApp, FusionBaseApi, getSettings, httpGet, route } from 'fusion-framework'
import { loadSettings } from './settings.mjs'

class UserModule extends FusionBaseApi {
  get() {
    return this.response({ mode: 'convention' })
  }

  UserAction() {
    return this.response({ mode: 'custom', action: 'user' })
  }
}

// Attach custom HTTP route metadata to the method (Node has no @decorator sugar).
httpGet('test/[action]', { title: 'User action' })(UserModule.prototype.UserAction)

route('/api/[module]', { tags: ['users'] })(UserModule)

await loadSettings()
const app = new FusionApp(getSettings())
await app.listen()
