/**
 * Tokio background tasks — usage example.
 *
 * Process-wide fire-and-forget / delayed jobs (not a durable queue).
 *
 *   node examples/background_tasks.mjs
 *
 * API:
 *   import { tasks } from 'fusion-framework'  // or require('../crates/fusion-node')
 *   tid = tasks.spawn(fn)
 *   tid = tasks.spawnAfter(ms, fn)
 *   tasks.cancel(tid)
 *   tasks.status(tid)  // pending|running|done|cancelled|failed
 */
import { createRequire } from 'node:module'
import { setTimeout as sleep } from 'node:timers/promises'

const require = createRequire(import.meta.url)
const { tasks } = require('../crates/fusion-node')

tasks.reset()
let n = 0
const work = () => {
  n += 1
  console.log('  work() ran')
}

// 1) Fire-and-forget
const tid = tasks.spawn(work)
console.log(`spawn     id=${tid}  status=${tasks.status(tid)}`)
for (let i = 0; i < 50; i++) {
  if (n >= 1 && tasks.status(tid) === 'done') break
  await sleep(20)
}
console.log(`          status=${tasks.status(tid)}  count=${n}`)

// 2) Delayed start, then let it finish
const delayed = tasks.spawnAfter(150, work)
console.log(`after     id=${delayed}  status=${tasks.status(delayed)}`)
for (let i = 0; i < 50; i++) {
  if (n >= 2 && tasks.status(delayed) === 'done') break
  await sleep(20)
}
console.log(`          status=${tasks.status(delayed)}  count=${n}`)

// 3) Cancel before the delay elapses
const doomed = tasks.spawnAfter(500, work)
console.log(`cancel    id=${doomed}  cancelled=${tasks.cancel(doomed)}`)
await sleep(200)
console.log(`          status=${tasks.status(doomed)}  count=${n} (unchanged)`)
