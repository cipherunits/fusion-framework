import './node_hello.mjs'
import { createRequire } from 'module'

const require = createRequire(import.meta.url)
const { FusionApp, getSettings, settings } = require('../crates/fusion-node/index.js')

// Global middleware (optional). Framework ships with none by default.
const MIDDLEWARE = []

settings.ensureLoaded([process.cwd()])
const app = new FusionApp(getSettings())
for (const mw of MIDDLEWARE) app.use(mw)
await app.listen()
