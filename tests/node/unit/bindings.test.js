const { describe, it, beforeEach } = require('node:test')
const assert = require('node:assert/strict')

const fusion = require('../helpers/load-fusion')
const {
  FusionBaseApi,
  FusionBaseTemplate,
  route,
  httpGet,
  clearRouteRegistry,
  openapiSpec,
  routeVersions,
  hasUnversionedRoutes,
  runMiddlewareChain,
  bearerJwt,
  cors,
  requireRoles,
  frameworkHeaders,
  staticFiles,
  resolveRoutePath,
  apiResourceName,
} = fusion

function handler(request) {
  return { status: 200, body: { state: request.state || {} } }
}

describe('routing helpers', () => {
  it('resolveRoutePath expands [module]', () => {
    assert.equal(resolveRoutePath('/api/[module]', { name: 'ProductModule' }), '/api/product')
  })

  it('apiResourceName strips Module suffix', () => {
    assert.equal(apiResourceName({ name: 'ProductModule' }), 'product')
  })
})

describe('http routes', () => {
  beforeEach(() => clearRouteRegistry())

  it('registers custom http_get with [action] token', () => {
    class UserModule extends FusionBaseApi {
      UserAction() {
        return { ok: true }
      }
    }
    httpGet('test/[action]')(UserModule.prototype.UserAction)
    route('/api/[module]')(UserModule)

    const spec = openapiSpec()
    assert.ok(spec.paths['/api/user/test/user'])
    assert.ok(spec.paths['/api/user/test/user'].get)
    assert.equal(spec.paths['/api/user/test/user'].get.operationId, 'UserModule_UserAction')
  })

  it('splits openapi specs by version', () => {
    class V1Hello extends FusionBaseApi {
      get() {
        return { v: 1 }
      }
    }
    class V2Hello extends FusionBaseApi {
      get() {
        return { v: 2 }
      }
    }
    class Health extends FusionBaseApi {
      get() {
        return { ok: true }
      }
    }

    route('/hello', { version: 'v1' })(V1Hello)
    route('/hello', { version: 'v2' })(V2Hello)
    route('/health')(Health)

    assert.deepEqual(routeVersions(), ['v1', 'v2'])
    assert.equal(hasUnversionedRoutes(), true)

    const v1 = openapiSpec('v1')
    assert.ok(v1.paths['/v1/hello'])
    assert.equal(v1.paths['/v2/hello'], undefined)
    assert.equal(v1.paths['/health'], undefined)
  })

  it('omits template routes from openapi', () => {
    class HomePage extends FusionBaseTemplate {
      static template = 'home/index.html'
      context() {
        return { title: 'Home' }
      }
    }
    class ItemsApi extends FusionBaseApi {
      get() {
        return { items: [] }
      }
    }

    route('/pages/home')(HomePage)
    route('/api/items', { version: 'v1', tags: ['items'] })(ItemsApi)

    const combined = openapiSpec()
    assert.equal(combined.paths['/pages/home'], undefined)

    const v1 = openapiSpec('v1')
    assert.equal(v1.paths['/pages/home'], undefined)
    assert.ok(v1.paths['/v1/api/items'])
  })
})

describe('middleware', () => {
  beforeEach(() => clearRouteRegistry())

  it('bearerJwt stores payload in state', async () => {
    const token = 'eyJhbGciOiJub25lIiwidHlwIjoiSldUIn0.eyJzdWIiOiIxIiwicm9sZXMiOlsiYWRtaW4iXX0.'
    const request = { headers: { Authorization: `Bearer ${token}` } }
    const result = await runMiddlewareChain(request, [bearerJwt()], handler)
    assert.equal(result.status, 200)
    assert.equal(result.body.state.jwt.sub, '1')
  })

  it('requireRoles returns 403 when role missing', async () => {
    const request = { headers: {}, state: { jwt: { roles: ['user'] } } }
    const result = await runMiddlewareChain(request, [requireRoles('admin')], handler)
    assert.equal(result.status, 403)
  })

  it('cors short-circuits OPTIONS preflight', async () => {
    const request = {
      method: 'OPTIONS',
      path: '/api',
      headers: { Origin: 'https://example.com' },
    }
    const result = await runMiddlewareChain(request, [cors()], handler)
    assert.equal(result.status, 204)
    const headers = Object.fromEntries(
      Object.entries(result.headers || {}).map(([k, v]) => [k.toLowerCase(), v])
    )
    assert.ok(headers['access-control-allow-origin'])
  })

  it('frameworkHeaders merges identity headers', async () => {
    const request = { path: '/', headers: {}, method: 'GET' }
    const result = await runMiddlewareChain(request, [frameworkHeaders()], handler)
    assert.ok(result.headers['x-powered-by'] || result.headers['X-Powered-By'])
  })

  it('staticFiles serves assets under prefix', async () => {
    const fs = require('fs')
    const os = require('os')
    const path = require('path')
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'fusion-static-'))
    const file = path.join(dir, 'logo.png')
    fs.writeFileSync(file, Buffer.from([0x89, 0x50, 0x4e, 0x47]))
    const request = { method: 'GET', path: '/static/logo.png', headers: {} }
    const result = await runMiddlewareChain(
      request,
      [staticFiles({ root: dir, prefix: '/static', maxAge: 60 })],
      handler,
    )
    assert.equal(result.status, 200)
    assert.ok(Buffer.isBuffer(result.body) || result.body instanceof Uint8Array)
    const headers = Object.fromEntries(
      Object.entries(result.headers || {}).map(([k, v]) => [k.toLowerCase(), v]),
    )
    assert.equal(headers['content-type'], 'image/png')
  })
})
