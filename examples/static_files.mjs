/**
 * Serve images/CSS with `staticFiles` (WhiteNoise-style).
 *
 *   app.use(staticFiles({ root: 'static', prefix: '/static' }))
 *   // <img src="/static/logo.png">
 */
import fs from 'fs'
import path from 'path'
import { fileURLToPath } from 'url'
import {
  FusionApp,
  FusionBaseApi,
  getSettings,
  route,
  staticFiles,
  status,
} from 'fusion-framework'

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const STATIC_DIR = path.join(__dirname, 'static_files_assets')

class Ping extends FusionBaseApi {
  get() {
    return this.response({ ok: true }, status.HTTP_SUCCESS)
  }
}

route('/api/ping')(Ping)

async function main() {
  fs.mkdirSync(STATIC_DIR, { recursive: true })
  const logo = path.join(STATIC_DIR, 'logo.png')
  if (!fs.existsSync(logo)) {
    fs.writeFileSync(logo, Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]))
  }

  await import('./settings.mjs').catch(() => {})
  const app = new FusionApp(getSettings())
  app.use(staticFiles({ root: STATIC_DIR, prefix: '/static' }))
  await app.listen()
}

main().catch((err) => {
  console.error(err)
  process.exit(1)
})
