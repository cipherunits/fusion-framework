/**
 * Application cache demo (sync + async, default driver: moka).
 *
 *   node examples/cache.mjs
 */
import { cache } from 'fusion-framework'

cache.configureDriver('moka', { defaultTtl: 60 })
cache.set('greeting', { hello: 'world' }, 30)
console.log('get:', cache.get('greeting'))
console.log('exists:', cache.exists('greeting'))
console.log('getOrSet:', cache.getOrSet('counter', () => 1))
console.log('existsOrSet (first):', cache.existsOrSet('flag', true))
console.log('existsOrSet (again):', cache.existsOrSet('flag', false))
console.log('deleteOrSet:', cache.deleteOrSet('greeting', { hello: 'fusion' }))
console.log('driver:', cache.driver())
cache.clear()
console.log('after clear:', cache.get('greeting'))

await cache.aset('async-greeting', { hello: 'async' }, 30)
console.log('aget:', await cache.aget('async-greeting'))
console.log(
  'agetOrSet:',
  await cache.agetOrSet('async-counter', async () => 1),
)
await cache.aclear()
console.log('after aclear:', await cache.aget('async-greeting'))
