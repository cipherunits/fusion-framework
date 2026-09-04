const { describe, it, beforeEach } = require('node:test')
const assert = require('node:assert/strict')
const { setTimeout: sleep } = require('node:timers/promises')

const { tasks } = require('../helpers/load-fusion')

async function waitDone(tid, check, { timeoutMs = 2000 } = {}) {
  const start = Date.now()
  while (Date.now() - start < timeoutMs) {
    if (check() && tasks.status(tid) === 'done') return
    await sleep(20)
  }
}

describe('background tasks', () => {
  beforeEach(() => {
    tasks.reset()
  })

  it('spawn runs to done', async () => {
    let n = 0
    const tid = tasks.spawn(() => {
      n += 1
    })
    await waitDone(tid, () => n === 1)
    assert.equal(n, 1)
    assert.equal(tasks.status(tid), 'done')
  })

  it('spawnAfter runs to done', async () => {
    let n = 0
    const tid = tasks.spawnAfter(80, () => {
      n += 1
    })
    assert.ok(['pending', 'running'].includes(tasks.status(tid)))
    await sleep(30)
    assert.equal(n, 0)
    await waitDone(tid, () => n === 1)
    assert.equal(n, 1)
    assert.equal(tasks.status(tid), 'done')
  })

  it('spawnAfter can be cancelled', async () => {
    let n = 0
    const tid = tasks.spawnAfter(400, () => {
      n += 1
    })
    assert.ok(['pending', 'running'].includes(tasks.status(tid)))
    assert.equal(tasks.cancel(tid), true)
    await sleep(150)
    assert.equal(n, 0)
    assert.equal(tasks.status(tid), 'cancelled')
  })

  it('status is null for unknown id', () => {
    assert.equal(tasks.status('task-does-not-exist'), null)
    assert.equal(tasks.cancel('task-does-not-exist'), false)
  })

  it('snapshot lists tasks', () => {
    const tid = tasks.spawnAfter(5000, () => {})
    const snap = tasks.snapshot()
    assert.ok(snap.task_count >= 1)
    assert.ok(snap.active_count >= 1)
    assert.ok(snap.tasks.some((t) => t.id === tid))
    assert.equal(tasks.cancel(tid), true)
    assert.equal(tasks.snapshot().tasks[0].status, 'cancelled')
  })
})
