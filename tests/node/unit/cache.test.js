const { describe, it, beforeEach } = require('node:test')
const assert = require('node:assert/strict')

const { cache } = require('../helpers/load-fusion')

describe('cache', () => {
  beforeEach(() => {
    cache.reset()
    cache.configureDriver('moka', { defaultTtl: null })
  })

  it('set/get/delete/exists', () => {
    assert.equal(cache.get('k'), null)
    cache.set('k', { n: 1 })
    assert.equal(cache.exists('k'), true)
    assert.deepEqual(cache.get('k'), { n: 1 })
    assert.equal(cache.delete('k'), true)
    assert.equal(cache.exists('k'), false)
  })

  it('getOrSet / existsOrSet / deleteOrSet', () => {
    let calls = 0
    assert.deepEqual(
      cache.getOrSet('x', () => {
        calls += 1
        return { v: calls }
      }),
      { v: 1 },
    )
    assert.deepEqual(
      cache.getOrSet('x', () => {
        calls += 1
        return { v: calls }
      }),
      { v: 1 },
    )
    assert.equal(calls, 1)
    assert.equal(cache.existsOrSet('f', true), false)
    assert.equal(cache.existsOrSet('f', false), true)
    assert.equal(cache.get('f'), true)
    assert.equal(cache.deleteOrSet('f', 'next'), 'next')
    assert.equal(cache.driver(), 'moka')
  })

  it('clear removes all keys', () => {
    cache.set('a', 1)
    cache.set('b', 2)
    cache.clear()
    assert.equal(cache.get('a'), null)
    assert.equal(cache.get('b'), null)
  })

  it('async aset/aget/aclear and agetOrSet', async () => {
    await cache.aset('async-k', { ok: true })
    assert.deepEqual(await cache.aget('async-k'), { ok: true })
    assert.equal(await cache.aexists('async-k'), true)
    let calls = 0
    assert.deepEqual(
      await cache.agetOrSet('ax', async () => {
        calls += 1
        return { v: calls }
      }),
      { v: 1 },
    )
    assert.deepEqual(
      await cache.agetOrSet('ax', async () => {
        calls += 1
        return { v: calls }
      }),
      { v: 1 },
    )
    assert.equal(calls, 1)
    await cache.aclear()
    assert.equal(await cache.aget('async-k'), null)
  })

  it('snapshot and panelContext', () => {
    cache.set('demo', { n: 1 })
    const snap = cache.snapshot()
    assert.equal(snap.driver, 'moka')
    assert.equal(snap.entry_count, 1)
    assert.equal(snap.entries[0].key, 'demo')
    assert.equal(snap.events[0].op, 'set')

    const ctx = cache.panelContext()
    assert.equal(ctx.title, 'Cache Monitor')
    assert.equal(ctx.empty_entries, false)
    assert.equal(ctx.entry_rows[0][0], 'demo')
    assert.match(String(ctx.json_path), /\/json$/)
  })
})
