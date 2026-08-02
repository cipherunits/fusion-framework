const path = require('path')
const fs = require('fs')
const { platform, arch } = process

function napiTriple() {
  const plat =
    platform === 'win32' ? 'win32' : platform === 'darwin' ? 'darwin' : platform === 'linux' ? 'linux' : platform
  const cpu =
    arch === 'x64' ? 'x64' : arch === 'arm64' ? 'arm64' : arch === 'ia32' ? 'ia32' : arch

  if (plat === 'win32' && cpu === 'x64') return 'win32-x64-msvc'
  if (plat === 'darwin' && cpu === 'arm64') return 'darwin-arm64'
  if (plat === 'darwin' && cpu === 'x64') return 'darwin-x64'
  if (plat === 'linux' && cpu === 'x64') return 'linux-x64-gnu'
  if (plat === 'linux' && cpu === 'arm64') return 'linux-arm64-gnu'
  return `${plat}-${cpu}`
}

function loadNative() {
  const triple = napiTriple()
  const candidates = [
    path.join(__dirname, `fusion-node.${triple}.node`),
    path.join(__dirname, 'fusion-node.node'),
    path.join(__dirname, 'fusion_node.node'),
  ]
  for (const candidate of candidates) {
    if (fs.existsSync(candidate)) {
      return require(candidate)
    }
  }
  throw new Error(
    `fusion-framework native addon not found for ${triple}. ` +
      `Run \`npm run build\` in crates/fusion-node or install a published package.`,
  )
}

const native = loadNative()
const NativeApp = native.App

const HTTP_METHODS = ['get', 'post', 'put', 'patch', 'delete', 'head', 'options']
const registry = []

let settings = {
  host: '127.0.0.1',
  port: 3000,
  debug: false,
}

class FusionBaseApi {
  constructor(request) {
    this.request = request
  }

  get method() {
    return String(this.request.method || '').toUpperCase()
  }

  get path() {
    return String(this.request.path || '')
  }

  get body() {
    return String(this.request.body || '')
  }

  get headers() {
    return this.request.headers || {}
  }

  get params() {
    return this.request.params || {}
  }

  ok(body = '', status = 200, headers = {}) {
    return { status, body, headers }
  }

  json(data, status = 200) {
    return {
      status,
      body: JSON.stringify(data),
      headers: { 'content-type': 'application/json' },
    }
  }
}

function apiResourceName(cls) {
  const name = typeof cls === 'string' ? cls : cls.name
  if (name.endsWith('Api') && name.length > 3) return name.slice(0, -3)
  if (name.endsWith('API') && name.length > 3) return name.slice(0, -3)
  return name
}

function resolveRoutePath(routePath, ApiClass) {
  // [name] → class name without trailing "Api" (MyFirstApi → MyFirst)
  return routePath.replaceAll('[name]', apiResourceName(ApiClass))
}

function router(routePath) {
  return function decorate(ApiClass) {
    const resolved = resolveRoutePath(routePath, ApiClass)
    ApiClass.__fusion_path__ = resolved
    ApiClass.__fusion_path_template__ = routePath
    registry.push({ path: resolved, ApiClass })
    return ApiClass
  }
}

function configure(next = {}) {
  settings = { ...settings, ...next }
  return settings
}

function getSettings() {
  return { ...settings }
}

function definesMethod(ApiClass, methodName) {
  let current = ApiClass
  while (current && current !== Function.prototype) {
    if (current === FusionBaseApi) break
    if (Object.prototype.hasOwnProperty.call(current.prototype, methodName)) {
      return true
    }
    current = Object.getPrototypeOf(current)
  }
  return false
}

function extractParamNames(fn) {
  const src = Function.prototype.toString.call(fn)
  const match = src.match(/^[^(]*\(([^)]*)\)/)
  if (!match) return []
  return match[1]
    .split(',')
    .map((part) => part.trim())
    .filter(Boolean)
    .map((part) => part.replace(/=.*$/, '').trim())
    .filter((name) => name && name !== 'request')
}

function coerce(value) {
  if (/^-?\d+$/.test(value)) return Number(value)
  if (value === 'true') return true
  if (value === 'false') return false
  return value
}

function invokeApiMethod(ApiClass, methodName, request) {
  const instance = new ApiClass(request)
  const fn = instance[methodName]
  const params = request.params || {}
  const args = extractParamNames(fn).map((name) => {
    if (!(name in params)) {
      throw new Error(`missing path param '${name}'`)
    }
    return coerce(params[name])
  })
  const result = fn.apply(instance, args)
  if (result != null && typeof result.then === 'function') {
    throw new Error(
      `async ${methodName}() is not supported in fusion-node yet; use a sync method`,
    )
  }
  return result
}

class FusionApp {
  constructor(customSettings) {
    this.settings = { ...settings, ...(customSettings || {}) }
    this.engine = new NativeApp()
    this.mounted = false
  }

  mount() {
    if (this.mounted) return
    for (const { path: routePath, ApiClass } of registry) {
      for (const methodName of HTTP_METHODS) {
        if (!definesMethod(ApiClass, methodName)) continue
        const boundClass = ApiClass
        const boundMethod = methodName
        this.engine.route(methodName.toUpperCase(), routePath, (request) =>
          invokeApiMethod(boundClass, boundMethod, request),
        )
      }
    }
    this.mounted = true
  }

  async listen(host, port) {
    this.mount()
    const h = host ?? this.settings.host
    const p = port ?? this.settings.port
    if (this.settings.debug) {
      console.log(`fusion listening on http://${h}:${p}`)
    }
    await this.engine.listen(h, Number(p))
  }
}

async function run(settingsModulePath) {
  if (settingsModulePath) {
    const mod = await import(pathToFileUrl(settingsModulePath))
    configure({
      host: mod.HOST ?? settings.host,
      port: mod.PORT ?? settings.port,
      debug: mod.DEBUG ?? settings.debug,
    })
  }
  const app = new FusionApp()
  await app.listen()
}

function pathToFileUrl(filePath) {
  const resolved = path.resolve(filePath)
  const url = require('url').pathToFileURL(resolved).href
  return url
}

module.exports = {
  App: NativeApp,
  FusionApp,
  FusionBaseApi,
  router,
  apiResourceName,
  resolveRoutePath,
  configure,
  getSettings,
  run,
}
