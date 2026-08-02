import './node_hello.mjs'
import { createRequire } from 'module'
import path from 'path'
import { fileURLToPath } from 'url'

const require = createRequire(import.meta.url)
const { run } = require('../crates/fusion-node/index.js')

const __dirname = path.dirname(fileURLToPath(import.meta.url))
await run(path.join(__dirname, 'settings.mjs'))
