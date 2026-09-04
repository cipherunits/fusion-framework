/**
 * Built-in Fusion monitor demo (cache + background tasks).
 *
 * Enable via fusion.<env>.json:
 *   "monitor": { "enabled": true, "path": "/__fusion/monitor" }
 *   "cache": { "driver": "moka", "max_events": 50 }
 *
 *   node examples/monitor.mjs
 */
import { createRequire } from 'node:module'

const require = createRequire(import.meta.url)
const { cache, settings, tasks } = require('../crates/fusion-node')

settings.merge({
  monitor: {
    enabled: true,
    path: '/__fusion/monitor',
  },
  cache: {
    driver: 'moka',
    default_ttl: null,
    max_events: 50,
  },
})
cache.configure(settings)
cache.set('demo:user', { name: 'Ada' }, 60)

tasks.reset()
tasks.spawn(() => {})
tasks.spawnAfter(5000, () => {})

const snap = cache.snapshot()
console.log(`driver=${snap.driver} keys=${snap.entry_count} events=${snap.event_count}`)
console.log(
  `tasks active=${snap.tasks.active_count} total=${snap.tasks.task_count}`,
)
console.log(`open http://127.0.0.1:8080${snap.monitor.path} after listen()`)
