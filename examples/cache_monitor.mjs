/**
 * Built-in cache monitor demo.
 *
 * Enable via fusion.<env>.json:
 *   "cache": { "monitor": { "enabled": true, "path": "/__fusion/cache", "max_events": 50 } }
 * When enabled is false, FusionApp does not register HTML or /json endpoints.
 *
 *   node examples/cache_monitor.mjs
 */
import { cache, settings } from 'fusion-framework'

settings.merge({
  cache: {
    driver: 'moka',
    default_ttl: null,
    monitor: {
      enabled: true,
      path: '/__fusion/cache',
      max_events: 50,
    },
  },
})
cache.configure(settings)
cache.set('demo:user', { name: 'Ada' }, 60)

const snap = cache.snapshot()
console.log(`driver=${snap.driver} keys=${snap.entry_count} events=${snap.event_count}`)
console.log(`open http://127.0.0.1:8080${snap.monitor.path} after listen()`)

// import { FusionApp } from 'fusion-framework'
// const app = new FusionApp(settings)
// await app.listen()
