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
const NativeSettings = native.Settings

const HTTP_METHODS = native.getHttpMethods()
const settings = new NativeSettings()
const registry = []

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

  response(body = '', status = 200, headers = {}) {
    const out = { status, body, headers: { ...headers } }
    if (body !== null && typeof body !== 'string' && !Buffer.isBuffer(body)) {
      out.headers = { 'content-type': 'application/json', ...headers }
    }
    return out
  }
}

function apiResourceName(cls) {
  const name = typeof cls === 'string' ? cls : cls.name
  return native.apiResourceNameJs(name)
}

function resolveRoutePath(routePath, ApiClass) {
  return native.resolveRoutePathJs(routePath, ApiClass.name)
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
  settings.merge(next)
  return getSettings()
}

function getSettings() {
  settings.ensureLoaded()
  return {
    host: settings.host,
    port: settings.port,
    debug: settings.debug,
    env: settings.env,
  }
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

function invokeApiMethod(ApiClass, methodName, request) {
  const instance = new ApiClass(request)
  const fn = instance[methodName]
  const params = request.params || {}
  const args = extractParamNames(fn).map((name) => {
    if (!(name in params)) {
      throw new Error(`missing path param '${name}'`)
    }
    return native.coerceParamJs(String(params[name]), 'auto')
  })
  // May return a value or a Promise — native call_async awaits Promises.
  return fn.apply(instance, args)
}

class FusionApp {
  constructor(customSettings) {
    if (customSettings) settings.merge(customSettings)
    this.settings = getSettings()
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
        this.engine.route(methodName.toUpperCase(), routePath, async (request) =>
          Promise.resolve(invokeApiMethod(boundClass, boundMethod, request)),
        )
      }
    }
    this.mounted = true
  }

  async listen(host, port) {
    this.mount()
    const snapshot = getSettings()
    const h = host ?? snapshot.host
    const p = port ?? snapshot.port
    if (snapshot.debug) {
      console.log(`fusion listening on http://${h}:${p}`)
    }
    await this.engine.listen(h, Number(p))
  }
}

async function run(settingsModulePath) {
  settings.loadJson(null, null, [process.cwd()])
  if (settingsModulePath) {
    const mod = await import(pathToFileUrl(settingsModulePath))
    const overlay = {}
    if (mod.HOST !== undefined) overlay.host = mod.HOST
    if (mod.PORT !== undefined) overlay.port = mod.PORT
    if (mod.DEBUG !== undefined) overlay.debug = mod.DEBUG
    if (Object.keys(overlay).length) settings.merge(overlay)
  }
  const app = new FusionApp()
  await app.listen()
}

function pathToFileUrl(filePath) {
  const resolved = path.resolve(filePath)
  return require('url').pathToFileURL(resolved).href
}

module.exports = {
  App: NativeApp,
  Settings: NativeSettings,
  FusionApp,
  FusionBaseApi,
  router,
  apiResourceName,
  resolveRoutePath,
  configure,
  getSettings,
  settings,
  HTTP_METHODS,
  run,
}
