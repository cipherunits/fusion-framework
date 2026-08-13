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
let activeGlobalMiddleware = []

const status = Object.create(null)
if (typeof native.getHttpStatusCodes === 'function') {
  for (const entry of native.getHttpStatusCodes()) {
    status[entry.name] = entry.code
  }
} else {
  // Fallback if native addon is older
  Object.assign(status, {
    HTTP_SUCCESS: 200,
    HTTP_200_OK: 200,
    HTTP_201_CREATED: 201,
    HTTP_204_NO_CONTENT: 204,
    HTTP_400_BAD_REQUEST: 400,
    HTTP_401_UNAUTHORIZED: 401,
    HTTP_403_FORBIDDEN: 403,
    HTTP_404_NOT_FOUND: 404,
    HTTP_500_INTERNAL_SERVER_ERROR: 500,
  })
}

const header = Object.create(null)
if (typeof native.getHttpHeaderConstants === 'function') {
  for (const entry of native.getHttpHeaderConstants()) {
    header[entry.name] = entry.value
  }
} else {
  Object.assign(header, {
    CONTENT_TYPE: 'Content-Type',
    CONTENT_DISPOSITION: 'Content-Disposition',
    LOCATION: 'Location',
    AUTHORIZATION: 'Authorization',
    APPLICATION_JSON: 'application/json',
    APPLICATION_OCTET_STREAM: 'application/octet-stream',
    APPLICATION_PDF: 'application/pdf',
  })
}

function mergeHeaderMap(map) {
  return map && typeof map === 'object' ? { ...map } : {}
}

header.attachment = (filename) =>
  typeof native.headerAttachment === 'function'
    ? mergeHeaderMap(native.headerAttachment(String(filename)))
    : { [header.CONTENT_DISPOSITION]: `attachment; filename="${filename}"` }

header.inline = (filename) =>
  typeof native.headerInline === 'function'
    ? mergeHeaderMap(native.headerInline(filename == null ? null : String(filename)))
    : filename
      ? { [header.CONTENT_DISPOSITION]: `inline; filename="${filename}"` }
      : { [header.CONTENT_DISPOSITION]: 'inline' }

header.contentType = (mediaType, charset) =>
  typeof native.headerContentType === 'function'
    ? mergeHeaderMap(native.headerContentType(String(mediaType), charset == null ? null : String(charset)))
    : {
        [header.CONTENT_TYPE]: charset
          ? `${mediaType}; charset=${charset}`
          : String(mediaType),
      }

header.location = (url) =>
  typeof native.headerLocation === 'function'
    ? mergeHeaderMap(native.headerLocation(String(url)))
    : { [header.LOCATION]: String(url) }

header.cacheControl = (value) =>
  typeof native.headerCacheControl === 'function'
    ? mergeHeaderMap(native.headerCacheControl(String(value)))
    : { 'Cache-Control': String(value) }

header.download = (filename, mediaType) =>
  typeof native.headerDownload === 'function'
    ? mergeHeaderMap(
        native.headerDownload(
          String(filename),
          mediaType == null ? null : String(mediaType),
        ),
      )
    : {
        ...header.contentType(mediaType || header.APPLICATION_OCTET_STREAM),
        ...header.attachment(filename),
      }

header.fingerprint = () =>
  typeof native.getFingerprintHeaders === 'function'
    ? mergeHeaderMap(native.getFingerprintHeaders())
    : {
        'X-Powered-By': 'Fusion Framework',
        'X-Framework': 'Fusion',
        'X-Fusion-Version': '1.2.1',
      }

function isThenable(value) {
  return value != null && typeof value.then === 'function'
}

async function awaitMaybe(value) {
  // Nested thenables (middleware wrapping async handlers) — same idea as Python
  // awaiting coroutines before treating the value as an HTTP body.
  let current = value
  for (let i = 0; i < 8 && isThenable(current); i++) {
    current = await current
  }
  return current
}

function mergeResponseHeaders(result, extra) {
  if (!result || typeof result !== 'object') {
    return { status: 200, body: result, headers: { ...extra } }
  }
  const headers = { ...extra, ...(result.headers || {}) }
  return { ...result, headers }
}

function frameworkHeaders() {
  const extra = header.fingerprint()
  // Async so Promises from callNext are never stuffed into `body` as objects.
  return async (request, callNext) => {
    const result = await awaitMaybe(callNext(request))
    return mergeResponseHeaders(result, extra)
  }
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

  get query() {
    return this.request.query || {}
  }

  get state() {
    return this.request.state || {}
  }

  response(body = '', status = 200, headers = {}) {
    // Keep this helper thin: content-type inference lives in fusion-core.
    const out = { status, body }
    const keys = headers ? Object.keys(headers) : []
    if (keys.length) out.headers = { ...headers }
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

function ensureState(request) {
  if (!request.state || typeof request.state !== 'object') {
    request.state = {}
  }
  return request.state
}

function isResponse(value) {
  return value && typeof value === 'object' && 'status' in value
}

async function runMiddlewareChain(request, middlewares, handler) {
  ensureState(request)

  async function dispatch(i, req) {
    if (i >= middlewares.length) {
      return await awaitMaybe(handler(req))
    }
    const middleware = middlewares[i]
    const callNext = (nextReq) => dispatch(i + 1, nextReq)
    const result = await awaitMaybe(middleware(req, callNext))
    if (isResponse(result)) return result
    return result
  }

  return dispatch(0, request)
}

function requireRoles(...rolesOrOptions) {
  let roles = rolesOrOptions
  let claim = 'roles'
  let stateKey = 'jwt'
  if (
    rolesOrOptions.length === 1 &&
    rolesOrOptions[0] &&
    typeof rolesOrOptions[0] === 'object' &&
    !Array.isArray(rolesOrOptions[0])
  ) {
    const opts = rolesOrOptions[0]
    roles = Array.isArray(opts.roles) ? opts.roles : []
    if (opts.claim) claim = opts.claim
    if (opts.stateKey) stateKey = opts.stateKey
  }
  const allowed = new Set(roles.map(String))
  return (request, callNext) => {
    const payload = ensureState(request)[stateKey]
    if (!payload) {
      return { status: 401, body: { detail: 'Authentication required' } }
    }
    let userRoles = payload[claim]
    if (userRoles == null) {
      return { status: 403, body: { detail: `Missing '${claim}' claim` } }
    }
    if (typeof userRoles === 'string') userRoles = [userRoles]
    if (!Array.isArray(userRoles)) {
      return { status: 403, body: { detail: `Invalid '${claim}' claim` } }
    }
    const hasRole = userRoles.some((r) => allowed.has(String(r)))
    if (!hasRole) {
      return { status: 403, body: { detail: 'Insufficient permissions', required: [...allowed] } }
    }
    return callNext(request)
  }
}

function bearerJwt(options = {}) {
  const stateKey = options.stateKey || 'jwt'
  const headerName = options.header || 'Authorization'
  const verify = typeof options.verify === 'function' ? options.verify : null

  return (request, callNext) => {
    const headers = request.headers || {}
    const auth =
      headers[headerName] || headers[headerName.toLowerCase()] || headers[headerName.toUpperCase()]
    if (!auth || !String(auth).toLowerCase().startsWith('bearer ')) {
      return { status: 401, body: { detail: 'Missing bearer token' } }
    }
    const token = String(auth).slice(7).trim()
    try {
      let payload
      if (verify) {
        payload = verify(token)
        if (!payload || typeof payload !== 'object') {
          return { status: 401, body: { detail: 'Invalid token' } }
        }
      } else {
        const parts = token.split('.')
        if (parts.length !== 3) throw new Error('bad token')
        const payloadB64 = parts[1] + '='.repeat((4 - (parts[1].length % 4)) % 4)
        payload = JSON.parse(Buffer.from(payloadB64, 'base64url').toString('utf8'))
      }
      ensureState(request)[stateKey] = payload
      return callNext(request)
    } catch {
      return { status: 401, body: { detail: 'Invalid token' } }
    }
  }
}

function router(routePath, options = {}) {
  return function decorate(ApiClass) {
    const resolvedBase = resolveRoutePath(routePath, ApiClass)

    const v = (options.version ?? '').toString().trim()
    const resolved =
      v.length > 0 ? `${v}/${resolvedBase.replace(/^\/+/, '')}` : resolvedBase

    ApiClass.__fusion_path__ = resolved
    ApiClass.__fusion_path_template__ = routePath

    const routeMiddleware = Array.isArray(options.middleware) ? [...options.middleware] : []
    if (Array.isArray(options.roles) && options.roles.length) {
      routeMiddleware.push(
        requireRoles({
          roles: options.roles,
          claim: options.roleClaim || 'roles',
          stateKey: options.roleStateKey || 'jwt',
        }),
      )
    }

    registry.push({
      path: resolved,
      ApiClass,
      middleware: routeMiddleware,
      swagger: {
        tags: Array.isArray(options.tags) ? options.tags : [],
        description: options.desc ?? null,
        title: options.title ?? null,
        deprecated: !!options.deprecated,
      },
      version_prefix: v,
    })
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

class HTTPException extends Error {
  constructor(status, detail = null, headers = {}) {
    super(typeof detail === 'string' ? detail : `HTTP ${status}`)
    this.status = Number(status)
    this.detail = detail == null ? '' : detail
    this.headers = headers || {}
  }

  toResponse() {
    const headers = { ...this.headers }
    const body = this.detail
    // Keep this helper thin: content-type inference lives in fusion-core.
    const out = { status: this.status, body }
    const keys = headers ? Object.keys(headers) : []
    if (keys.length) out.headers = headers
    return out
  }
}

function asObject(value) {
  return value && typeof value === 'object' && !Array.isArray(value) ? value : {}
}

function asList(value) {
  return Array.isArray(value) ? value : []
}

function truthyEnabled(value, defaultValue = true) {
  if (value === undefined || value === null) return defaultValue
  if (value === false || value === 0 || value === 'false' || value === '0' || value === 'off' || value === 'no') {
    return false
  }
  if (value === true || value === 1 || value === 'true' || value === '1' || value === 'on' || value === 'yes') {
    return true
  }
  return Boolean(value)
}

function readSwaggerSettings() {
  if (!truthyEnabled(settings.get('swagger.enabled', true))) {
    return { enabled: false }
  }

  let pathValue = settings.get('swagger.path', '/swagger')
  if (pathValue === false || pathValue === null || pathValue === '' || pathValue === 'false' || pathValue === 'off') {
    return { enabled: false }
  }

  let prefix = String(pathValue).replace(/\/+$/, '') || '/swagger'
  if (!prefix.startsWith('/')) prefix = `/${prefix}`

  const info = asObject(settings.get('swagger.info', {}))
  for (const key of ['title', 'version', 'description', 'termsOfService', 'contact', 'license']) {
    const flat = settings.get(`swagger.${key}`, undefined)
    if (flat !== undefined && flat !== null && info[key] === undefined) info[key] = flat
  }
  if (!info.title) info.title = 'fusion-framework'
  if (!info.version) info.version = '1.0.0'

  const pageTitle = settings.get('swagger.title', null) || info.title || 'Fusion API Docs'

  const authRaw = asObject(settings.get('swagger.auth', {}))
  const schemes = asObject(authRaw.schemes)
  const oauth = asObject(authRaw.oauth)
  const globalSecurity = asList(authRaw.global)
  let persistAuth = authRaw.persistAuthorization
  if (persistAuth === undefined) persistAuth = false

  const navbarRaw = asObject(settings.get('swagger.navbar', {}))
  const navbar = {
    enabled: truthyEnabled(navbarRaw.enabled, true),
    showUrlInput: truthyEnabled(navbarRaw.showUrlInput, true),
    urls: Array.isArray(navbarRaw.urls) ? navbarRaw.urls : null,
  }

  const ui = {
    deepLinking: true,
    displayOperationId: false,
    defaultModelsExpandDepth: 1,
    defaultModelExpandDepth: 1,
    defaultModelRendering: 'example',
    docExpansion: 'list',
    filter: true,
    tryItOutEnabled: true,
    persistAuthorization: Boolean(persistAuth),
    displayRequestDuration: true,
    showExtensions: false,
    showCommonExtensions: false,
    syntaxHighlight: { activated: true, theme: 'agate' },
    withCredentials: false,
    validatorUrl: 'https://validator.swagger.io/validator',
    ...asObject(settings.get('swagger.ui', {})),
  }
  if (Object.prototype.hasOwnProperty.call(authRaw, 'persistAuthorization')) {
    ui.persistAuthorization = Boolean(persistAuth)
  }

  let servers = settings.get('swagger.servers', null)
  if (!Array.isArray(servers)) servers = []

  return {
    enabled: true,
    path: prefix,
    pageTitle: String(pageTitle),
    info,
    servers,
    auth: {
      schemes,
      global: globalSecurity,
      oauth,
      persistAuthorization: Boolean(persistAuth),
    },
    navbar,
    ui,
  }
}

function applySwaggerOpenApi(openapi, swagger) {
  openapi.info = { ...asObject(openapi.info), ...swagger.info }
  if (swagger.servers?.length) openapi.servers = swagger.servers
  if (swagger.auth?.schemes && Object.keys(swagger.auth.schemes).length) {
    openapi.components = asObject(openapi.components)
    openapi.components.securitySchemes = {
      ...asObject(openapi.components.securitySchemes),
      ...swagger.auth.schemes,
    }
  }
  if (swagger.auth?.global?.length) openapi.security = swagger.auth.global
  return openapi
}

function swaggerUiHtml(swagger, openapiUrl) {
  const uiOpts = { ...swagger.ui }
  delete uiOpts.presets
  delete uiOpts.plugins
  delete uiOpts.layout

  if (swagger.navbar?.urls?.length) {
    delete uiOpts.url
    uiOpts.urls = swagger.navbar.urls
  } else {
    uiOpts.url = openapiUrl
    delete uiOpts.urls
  }
  uiOpts.dom_id = '#swagger-ui'

  const uiJson = JSON.stringify(uiOpts).replace(/</g, '\\u003c')
  const oauth = swagger.auth?.oauth && Object.keys(swagger.auth.oauth).length ? swagger.auth.oauth : null
  const oauthJson = JSON.stringify(oauth).replace(/</g, '\\u003c')
  const title = String(swagger.pageTitle)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')

  const navbarEnabled = !!swagger.navbar?.enabled
  const showUrlInput = swagger.navbar?.showUrlInput !== false
  const hideUrlCss =
    navbarEnabled && !showUrlInput
      ? `<style>.topbar .download-url-wrapper { display: none !important; }</style>`
      : ''
  const standaloneScript = navbarEnabled
    ? `<script src="https://unpkg.com/swagger-ui-dist/swagger-ui-standalone-preset.js"></script>`
    : ''

  return `<!doctype html>
<html>
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>${title}</title>
    <link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist/swagger-ui.css" />
    ${hideUrlCss}
  </head>
  <body>
    <div id="swagger-ui"></div>
    <script src="https://unpkg.com/swagger-ui-dist/swagger-ui-bundle.js"></script>
    ${standaloneScript}
    <script>
      window.onload = function() {
        var opts = ${uiJson};
        opts.presets = [SwaggerUIBundle.presets.apis];
        opts.plugins = [SwaggerUIBundle.plugins.DownloadUrl];
        if (${navbarEnabled ? 'true' : 'false'} && typeof SwaggerUIStandalonePreset !== 'undefined') {
          opts.presets.push(SwaggerUIStandalonePreset);
          opts.layout = 'StandaloneLayout';
        } else {
          opts.layout = 'BaseLayout';
        }
        var ui = SwaggerUIBundle(opts);
        var oauth = ${oauthJson};
        if (oauth && typeof ui.initOAuth === 'function') {
          ui.initOAuth(oauth);
        }
        window.ui = ui;
      };
    </script>
  </body>
</html>`
}

class FusionApp {
  constructor(customSettings) {
    if (customSettings) settings.merge(customSettings)
    this.settings = getSettings()
    this.engine = new NativeApp()
    this.mounted = false
    // Default: advertise Fusion to clients / Wappalyzer-style detectors.
    this._middleware = [frameworkHeaders()]
  }

  use(middleware) {
    this._middleware.push(middleware)
  }

  mount() {
    if (this.mounted) return
    activeGlobalMiddleware = [...this._middleware]

    for (const { path: routePath, ApiClass, middleware: routeMiddleware = [] } of registry) {
      for (const methodName of HTTP_METHODS) {
        if (!definesMethod(ApiClass, methodName)) continue
        this.engine.route(methodName.toUpperCase(), routePath, async (request) => {
          const chain = [...activeGlobalMiddleware, ...routeMiddleware]
          const handler = async (req) => {
            try {
              const instance = new ApiClass(req)
              const fn = instance[methodName]
              return await Promise.resolve(fn.call(instance))
            } catch (err) {
              if (err instanceof HTTPException) return err.toResponse()
              throw err
            }
          }
          return runMiddlewareChain(request, chain, handler)
        })
      }
    }

    const swagger = readSwaggerSettings()
    if (swagger.enabled) {
      const prefix = swagger.path

      const openapi = applySwaggerOpenApi(
        {
          openapi: '3.0.3',
          info: { ...swagger.info },
          paths: {},
        },
        swagger,
      )

      const parsePathParams = (pattern) => {
        return String(pattern)
          .split('/')
          .filter((seg) => (seg.startsWith('{') && seg.endsWith('}')) || (seg.startsWith('[') && seg.endsWith(']')))
          .map((seg) => seg.slice(1, -1))
      }

      for (const item of registry) {
        const { path: p, ApiClass, swagger: routeSwagger } = item
        const pathParams = parsePathParams(p)
        const resolvedPath = p.startsWith('/') ? p : `/${p}`

        if (!openapi.paths[resolvedPath]) openapi.paths[resolvedPath] = {}

        for (const methodName of HTTP_METHODS) {
          if (!definesMethod(ApiClass, methodName)) continue

          const methodUpper = String(methodName).toUpperCase()
          const methodLower = String(methodName).toLowerCase()

          const params = pathParams.map((name) => ({
            name,
            in: 'path',
            required: true,
            schema: { type: 'string' },
          }))

          openapi.paths[resolvedPath][methodLower] = {
            tags: routeSwagger?.tags?.length ? routeSwagger.tags : [],
            summary: routeSwagger?.title ?? `${ApiClass.name}.${methodUpper}`,
            description: routeSwagger?.description ?? '',
            deprecated: !!routeSwagger?.deprecated,
            operationId: `${ApiClass.name}_${methodLower}`,
            parameters: params,
            responses: { '200': { description: 'OK' } },
          }
        }
      }

      this.engine.route('GET', `${prefix}/openapi.json`, async () => openapi)
      this.engine.route('GET', prefix, async () => ({
        status: 200,
        body: swaggerUiHtml(swagger, `${prefix}/openapi.json`),
        headers: { 'content-type': 'text/html' },
      }))
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

async function run(options = {}) {
  const settingsModulePath =
    typeof options === 'string' ? options : options && options.settingsModule
  const middleware = Array.isArray(options?.middleware) ? options.middleware : []

  settings.ensureLoaded([process.cwd()])
  if (settingsModulePath) {
    const mod = await import(pathToFileUrl(settingsModulePath))
    const overlay = {}
    if (mod.HOST !== undefined) overlay.host = mod.HOST
    if (mod.PORT !== undefined) overlay.port = mod.PORT
    if (mod.DEBUG !== undefined) overlay.debug = mod.DEBUG
    if (Object.keys(overlay).length) settings.merge(overlay)
  }
  const app = new FusionApp()
  for (const mw of middleware) app.use(mw)
  await app.listen()
  return app
}

function coerceParam(raw, kind = 'auto') {
  return native.coerceParamJs(String(raw), kind)
}

function pathToFileUrl(filePath) {
  const resolved = path.resolve(filePath)
  return require('url').pathToFileURL(resolved).href
}

const route = router

module.exports = {
  App: NativeApp,
  Settings: NativeSettings,
  FusionApp,
  FusionBaseApi,
  HTTPException,
  router,
  route,
  apiResourceName,
  resolveRoutePath,
  configure,
  getSettings,
  settings,
  status,
  header,
  HTTP_METHODS,
  run,
  bearerJwt,
  requireRoles,
  frameworkHeaders,
  runMiddlewareChain,
  coerceParam,
  getHttpMethods: () => HTTP_METHODS,
  apiResourceNameJs: native.apiResourceNameJs,
  resolveRoutePathJs: native.resolveRoutePathJs,
  coerceParamJs: native.coerceParamJs,
}
