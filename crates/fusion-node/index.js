const path = require('path')
const fs = require('fs')
const { spawn } = require('child_process')
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
        ['X-Fusion-Version']: '1.2.6',
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

function getHeader(request, name) {
  const headers = request.headers || {}
  const target = String(name).toLowerCase()
  for (const [key, value] of Object.entries(headers)) {
    if (String(key).toLowerCase() === target) return String(value)
  }
  return null
}

function headerMiddleware(extra) {
  return async (request, callNext) => {
    const result = await awaitMaybe(callNext(request))
    return mergeResponseHeaders(result, extra)
  }
}

function securityHeaders(options = {}) {
  const extra = {
    'X-Content-Type-Options': options.contentTypeOptions ?? 'nosniff',
    'X-Frame-Options': options.frameOptions ?? 'DENY',
    'Referrer-Policy': options.referrerPolicy ?? 'strict-origin-when-cross-origin',
    'Permissions-Policy':
      options.permissionsPolicy ?? 'camera=(), microphone=(), geolocation=(), payment=()',
    'Cross-Origin-Opener-Policy': options.coop ?? 'same-origin',
    'Cross-Origin-Resource-Policy': options.corp ?? 'same-origin',
  }
  if (options.csp) extra['Content-Security-Policy'] = String(options.csp)
  if (options.hsts) extra['Strict-Transport-Security'] = String(options.hsts)
  return headerMiddleware(extra)
}

function cacheHeaders(options = {}) {
  return headerMiddleware({
    'Cache-Control': options.default ?? options.value ?? 'no-store',
  })
}

function requestId(options = {}) {
  const headerName = options.header ?? 'X-Request-Id'
  const incoming = options.incoming !== false
  return async (request, callNext) => {
    const state = ensureState(request)
    let rid = incoming ? getHeader(request, headerName) : null
    if (!rid) {
      rid =
        typeof crypto !== 'undefined' && crypto.randomUUID
          ? crypto.randomUUID()
          : `${Date.now()}-${Math.random().toString(16).slice(2)}`
    }
    state.request_id = rid
    const result = await awaitMaybe(callNext(request))
    return mergeResponseHeaders(result, { [headerName]: rid })
  }
}

function cors(options = {}) {
  const origins = Array.isArray(options.allowOrigins)
    ? options.allowOrigins.map(String)
    : [String(options.allowOrigins ?? '*')]
  const methods = (
    options.allowMethods ?? ['GET', 'POST', 'PUT', 'PATCH', 'DELETE', 'OPTIONS', 'HEAD']
  ).map((m) => String(m).toUpperCase())
  const allowHeaders = (
    options.allowHeaders ?? ['Authorization', 'Content-Type', 'Accept', 'Origin', 'X-Request-Id']
  ).map(String)
  const exposeHeaders = (options.exposeHeaders ?? ['X-Request-Id']).map(String)
  const allowCredentials = !!options.allowCredentials
  const maxAge = Number(options.maxAge ?? 600)
  const allowAll = origins.includes('*')

  function corsHeaders(origin) {
    let chosen = '*'
    if (!allowAll) {
      if (origin && origins.includes(origin)) chosen = origin
      else if (origins.length) chosen = origins[0]
    }
    const out = {
      'Access-Control-Allow-Origin': chosen,
      'Access-Control-Allow-Methods': methods.join(', '),
      'Access-Control-Allow-Headers': allowHeaders.join(', '),
      'Access-Control-Expose-Headers': exposeHeaders.join(', '),
      'Access-Control-Max-Age': String(maxAge),
      Vary: 'Origin',
    }
    if (allowCredentials && chosen !== '*') out['Access-Control-Allow-Credentials'] = 'true'
    return out
  }

  return async (request, callNext) => {
    const origin = getHeader(request, 'Origin')
    const extra = corsHeaders(origin)
    if (String(request.method || 'GET').toUpperCase() === 'OPTIONS') {
      return { status: 204, body: '', headers: extra }
    }
    const result = await awaitMaybe(callNext(request))
    return mergeResponseHeaders(result, extra)
  }
}

const STATIC_MIME_TYPES = {
  '.css': 'text/css; charset=utf-8',
  '.gif': 'image/gif',
  '.htm': 'text/html; charset=utf-8',
  '.html': 'text/html; charset=utf-8',
  '.ico': 'image/x-icon',
  '.jpeg': 'image/jpeg',
  '.jpg': 'image/jpeg',
  '.js': 'text/javascript; charset=utf-8',
  '.json': 'application/json',
  '.map': 'application/json',
  '.png': 'image/png',
  '.svg': 'image/svg+xml',
  '.txt': 'text/plain; charset=utf-8',
  '.webp': 'image/webp',
  '.woff': 'font/woff',
  '.woff2': 'font/woff2',
}

/** Guess Content-Type from a file path extension. */
function guessStaticContentType(filePath) {
  const ext = path.extname(String(filePath)).toLowerCase()
  return STATIC_MIME_TYPES[ext] || 'application/octet-stream'
}

/**
 * Serve files from `root` for URLs under `prefix` (WhiteNoise-style).
 *
 * - root: folder on disk (e.g. 'static')
 * - prefix: URL prefix (e.g. '/static' → static/logo.png at /static/logo.png)
 *
 * Files are also mounted as real GET/HEAD routes on FusionApp.mount()/listen().
 */
function staticFiles(options = {}) {
  const rootDir = path.resolve(String(options.root ?? 'static'))
  const rawPrefix = String(options.prefix ?? '/static').trim()
  const normalized = rawPrefix.replace(/\/+$/, '') === '' ? '/' : `/${rawPrefix.replace(/^\/+|\/+$/g, '')}`
  const maxAge = options.maxAge === undefined ? 3600 : options.maxAge
  const allowFallthrough =
    options.fallthrough === undefined ? normalized === '/' : !!options.fallthrough
  const cfg = { root: rootDir, prefix: normalized, maxAge, fallthrough: allowFallthrough }

  const middleware = (request, callNext) => serveStaticOrNext(cfg, request, callNext)
  middleware.__fusionStatic = cfg
  return middleware
}

/** Build a 200 file response envelope. */
function staticFileResponse(filePath, method, maxAge) {
  const size = fs.statSync(filePath).size
  const headers = {
    'content-type': guessStaticContentType(filePath),
    'content-length': String(size),
  }
  if (maxAge !== null && maxAge !== undefined) {
    headers['cache-control'] = `public, max-age=${Number(maxAge)}`
  }
  const body = String(method).toUpperCase() === 'HEAD' ? Buffer.alloc(0) : fs.readFileSync(filePath)
  return { status: 200, body, headers }
}

/** Try to serve a static file; otherwise callNext. */
function serveStaticOrNext(cfg, request, callNext) {
  const method = String(request.method || 'GET').toUpperCase()
  if (method !== 'GET' && method !== 'HEAD') return callNext(request)

  const reqPath = String(request.path || '/')
  const normalized = cfg.prefix
  let relative = ''
  if (normalized === '/') {
    relative = reqPath.replace(/^\/+/, '')
    if (!relative || relative.endsWith('/')) return callNext(request)
  } else {
    if (!(reqPath === normalized || reqPath.startsWith(`${normalized}/`))) {
      return callNext(request)
    }
    relative = reqPath.slice(normalized.length).replace(/^\/+/, '')
    if (!relative) return callNext(request)
  }

  const candidate = path.resolve(cfg.root, relative)
  const relToRoot = path.relative(cfg.root, candidate)
  if (relToRoot.startsWith('..') || path.isAbsolute(relToRoot)) {
    return { status: 403, body: { detail: 'Forbidden' } }
  }
  if (!fs.existsSync(candidate) || !fs.statSync(candidate).isFile()) {
    if (cfg.fallthrough) return callNext(request)
    return { status: 404, body: { detail: 'Not found' } }
  }
  return staticFileResponse(candidate, method, cfg.maxAge)
}

/** Register GET/HEAD routes for files under each staticFiles() mount. */
function mountStaticFiles(engine, middlewares) {
  for (const mw of middlewares || []) {
    const cfg = mw && mw.__fusionStatic
    if (!cfg || !fs.existsSync(cfg.root) || !fs.statSync(cfg.root).isDirectory()) continue
    const walk = (dir) => {
      for (const name of fs.readdirSync(dir)) {
        const full = path.join(dir, name)
        const st = fs.statSync(full)
        if (st.isDirectory()) {
          walk(full)
          continue
        }
        if (!st.isFile()) continue
        const rel = path.relative(cfg.root, full).split(path.sep).join('/')
        const url = cfg.prefix === '/' ? `/${rel}` : `${cfg.prefix}/${rel}`
        engine.route('GET', url, () => staticFileResponse(full, 'GET', cfg.maxAge))
        engine.route('HEAD', url, () => staticFileResponse(full, 'HEAD', cfg.maxAge))
      }
    }
    walk(cfg.root)
  }
}

class FusionBaseApi {
  constructor(request) {
    this.request = request && typeof request === 'object' ? request : emptyRequest()
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

  pagination({
    page,
    pageSize,
    offset,
    defaultPageSize = 20,
    maxPageSize = 100,
  } = {}) {
    const query = { ...(this.query || {}) }
    if (page != null) query.page = String(page)
    if (pageSize != null) query.page_size = String(pageSize)
    if (offset != null) query.offset = String(offset)
    return parsePagination(query, { defaultPageSize, maxPageSize })
  }

  paginated(items, total, params = null, { page, pageSize, status = 200, headers } = {}) {
    const p = params ?? this.pagination({ page, pageSize })
    const body = paginatedBody(items, total, p)
    return this.response(body, status, headers || {})
  }

  wantsJson() {
    let accept = null
    for (const [key, value] of Object.entries(this.headers || {})) {
      if (key.toLowerCase() === 'accept') {
        accept = String(value)
        break
      }
    }
    const format = this.query?.format != null ? String(this.query.format) : null
    return typeof native.prefersJsonJs === 'function'
      ? native.prefersJsonJs(accept, format)
      : prefersJsonFallback(accept, format)
  }
}

function prefersJsonFallback(accept, formatQuery) {
  if (formatQuery && String(formatQuery).toLowerCase() === 'json') return true
  const value = String(accept || '').trim().toLowerCase()
  if (!value) return false
  let bestJson = -1
  let bestHtml = -1
  for (const part of value.split(',')) {
    const tokens = part.trim().split(';').map((t) => t.trim())
    const media = tokens[0] || ''
    let q = 1
    for (const token of tokens.slice(1)) {
      if (token.startsWith('q=')) {
        const parsed = Number.parseFloat(token.slice(2))
        if (!Number.isNaN(parsed)) q = parsed
      }
    }
    if (media === 'application/json' || media === 'text/json') bestJson = Math.max(bestJson, q)
    else if (media === 'text/html' || media === 'application/xhtml+xml') bestHtml = Math.max(bestHtml, q)
  }
  return bestJson > 0 && bestJson >= bestHtml
}

class FusionBaseTemplate extends FusionBaseApi {
  static __fusion_template__ = true
  static template = ''
  static templateAddress = ''
  static templatesDir = ''

  /** Template variables; may return a Promise (async context). */
  context() {
    return {}
  }

  get() {
    const raw = this.context()
    if (raw && typeof raw.then === 'function') {
      return this._getAsync(raw)
    }
    return this._finishGet(raw)
  }

  async _getAsync(raw) {
    const ctx = await raw
    return this._finishGet(ctx)
  }

  _finishGet(ctx) {
    const data = { ...(ctx || {}) }
    if (this.wantsJson()) return data
    return this._htmlResponse(data)
  }

  templateName() {
    const name = this.constructor.template || this.constructor.templateAddress
    if (!name) {
      throw new Error(`${this.constructor.name} must set static template or templateAddress`)
    }
    return name
  }

  templatesRoot() {
    if (this.constructor.templatesDir) return this.constructor.templatesDir
    return String(settings.get('templates.dir', 'templates'))
  }

  render({
    status = 200,
    headers = {},
    context = null,
    templateName = null,
  } = {}) {
    const raw = this.context()
    if (raw && typeof raw.then === 'function') {
      return this._renderAsync(raw, { status, headers, context, templateName })
    }
    const ctx = { ...(raw || {}), ...(context || {}) }
    return this._htmlResponse(ctx, { status, headers, templateName })
  }

  async _renderAsync(raw, { status = 200, headers = {}, context = null, templateName = null } = {}) {
    const base = await raw
    const ctx = { ...(base || {}), ...(context || {}) }
    return this._htmlResponse(ctx, { status, headers, templateName })
  }

  _htmlResponse(ctx, { status = 200, headers = {}, templateName = null } = {}) {
    const html = renderTemplate(
      templateName || this.templateName(),
      ctx,
      this.templatesRoot(),
    )
    return this.response(html, status, {
      'content-type': 'text/html; charset=utf-8',
      ...headers,
    })
  }
}

function renderTemplate(templateName, context = {}, templatesRoot = null) {
  const root = templatesRoot ?? String(settings.get('templates.dir', 'templates'))
  return native.renderTemplateJs(templateName, context || {}, root)
}

function apiResourceName(cls) {
  const name = typeof cls === 'string' ? cls : cls.name
  return native.apiResourceNameJs(name)
}

function resolveRoutePath(routePath, ApiClass) {
  return native.resolveRoutePathJs(routePath, ApiClass.name)
}

function apiActionName(methodName) {
  if (typeof native.apiActionNameJs === 'function') {
    return native.apiActionNameJs(methodName)
  }
  let stem = methodName
  if (methodName.endsWith('Action') && methodName.length > 6) stem = methodName.slice(0, -6)
  else if (methodName.endsWith('ACTION') && methodName.length > 6) stem = methodName.slice(0, -6)
  return stem.toLowerCase()
}

function resolveMethodRoutePath(template, className, methodName) {
  if (typeof native.resolveMethodRoutePathJs === 'function') {
    return native.resolveMethodRoutePathJs(template, className, methodName)
  }
  return resolveRoutePath(template, { name: className }).replace(
    /\[action\]/g,
    apiActionName(methodName),
  )
}

function joinRoutePaths(base, segment) {
  const left = String(base || '').replace(/\/+$/, '')
  const right = String(segment || '').replace(/^\/+|\/+$/g, '')
  if (!right) return left || '/'
  if (!left) return `/${right}`
  return `${left}/${right}`
}

function resolveHandlerRoute(classBasePath, template, className, methodName) {
  if (typeof native.resolveHandlerRouteJs === 'function') {
    return native.resolveHandlerRouteJs(classBasePath, template, className, methodName)
  }
  const resolved = resolveMethodRoutePath(template, className, methodName)
  if (String(template).startsWith('/')) {
    return joinRoutePaths('', resolved.replace(/^\/+/, ''))
  }
  return joinRoutePaths(classBasePath, resolved)
}

function httpRoute(method, route, options = {}) {
  // tags unset → inherit class route tags; tags: [] clears; tags: [...] overrides.
  const tagsSet = Object.prototype.hasOwnProperty.call(options, 'tags')
  const meta = {
    method: String(method).toLowerCase(),
    template: route,
    tagsSet,
    tags: tagsSet ? (Array.isArray(options.tags) ? options.tags : []) : null,
    desc: options.desc ?? null,
    title: options.title ?? null,
    deprecated: !!options.deprecated,
  }
  function wrap(fn) {
    fn.__fusionHttpRoute = meta
    return fn
  }
  return wrap
}

function mergeSwagger(methodSwagger, classSwagger) {
  return {
    tags: methodSwagger.tagsSet ? methodSwagger.tags : classSwagger.tags,
    description: methodSwagger.desc ?? classSwagger.description,
    title: methodSwagger.title ?? classSwagger.title,
    deprecated: !!(methodSwagger.deprecated || classSwagger.deprecated),
  }
}

function httpGet(route, options = {}) {
  return httpRoute('get', route, options)
}
function httpPost(route, options = {}) {
  return httpRoute('post', route, options)
}
function httpPut(route, options = {}) {
  return httpRoute('put', route, options)
}
function httpPatch(route, options = {}) {
  return httpRoute('patch', route, options)
}
function httpDelete(route, options = {}) {
  return httpRoute('delete', route, options)
}
function httpHead(route, options = {}) {
  return httpRoute('head', route, options)
}
function httpOptions(route, options = {}) {
  return httpRoute('options', route, options)
}

function collectRouteSlots(ApiClass, classBasePath, classSwagger) {
  const slots = []
  const custom = new Set()
  const className = ApiClass.name

  for (const key of Object.getOwnPropertyNames(ApiClass.prototype)) {
    if (key === 'constructor' || key.startsWith('_')) continue
    const fn = ApiClass.prototype[key]
    if (typeof fn !== 'function' || !fn.__fusionHttpRoute) continue
    custom.add(key)
    const meta = fn.__fusionHttpRoute
    slots.push({
      path: resolveHandlerRoute(classBasePath, meta.template, className, key),
      httpMethod: meta.method,
      handlerMethod: key,
      swagger: mergeSwagger(meta, classSwagger),
    })
  }

  for (const methodName of HTTP_METHODS) {
    if (custom.has(methodName)) continue
    if (!definesMethod(ApiClass, methodName)) continue
    slots.push({
      path: classBasePath,
      httpMethod: methodName,
      handlerMethod: methodName,
      swagger: classSwagger,
    })
  }

  return slots
}

function emptyRequest() {
  return {
    method: '',
    path: '',
    body: '',
    headers: {},
    params: {},
    query: {},
    state: {},
  }
}

/**
 * N-API threadsafe callbacks may be `(request)` (Fatal) or `(err, request)` (CalleeHandled).
 * Always return a real request object so middleware never sees `null`.
 */
function nativeRequestArg(errOrRequest, maybeRequest) {
  if (maybeRequest != null && typeof maybeRequest === 'object') {
    return maybeRequest
  }
  if (errOrRequest instanceof Error) {
    throw errOrRequest
  }
  if (errOrRequest != null && typeof errOrRequest === 'object') {
    return errOrRequest
  }
  return emptyRequest()
}

function ensureState(request) {
  if (!request || typeof request !== 'object') {
    return {}
  }
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

function requirePermissions(...checks) {
  return (request, callNext) => {
    for (const check of checks) {
      if (!check(request)) {
        return { status: 403, body: { detail: 'Forbidden' } }
      }
    }
    return callNext(request)
  }
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
    const permissionChecks = Array.isArray(options.permissions) ? options.permissions : []
    if (permissionChecks.length) {
      routeMiddleware.push(requirePermissions(...permissionChecks))
    }

    const classSwagger = {
      tags: Array.isArray(options.tags) ? options.tags : [],
      description: options.desc ?? null,
      title: options.title ?? null,
      deprecated: !!options.deprecated,
    }

    registry.push({
      path: resolved,
      classBasePath: resolved,
      ApiClass,
      middleware: routeMiddleware,
      swagger: classSwagger,
      version_prefix: v,
      requiresPermissions: permissionChecks.length > 0,
      slots: collectRouteSlots(ApiClass, resolved, classSwagger),
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
    showUrlInputSet: Object.prototype.hasOwnProperty.call(navbarRaw, 'showUrlInput'),
    urls: Array.isArray(navbarRaw.urls) ? navbarRaw.urls : null,
    urlsSet: Array.isArray(navbarRaw.urls),
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
    validatorUrl: null,
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

const UNVERSIONED_SWAGGER_NAME = 'default'

const SWAGGER_ASSETS_DIR = path.join(__dirname, 'static', 'swagger-ui')
const SWAGGER_ASSET_TYPES = {
  'swagger-ui-bundle.js': 'application/javascript; charset=utf-8',
  'swagger-ui-standalone-preset.js': 'application/javascript; charset=utf-8',
  'swagger-ui.css': 'text/css; charset=utf-8',
}

function loadSwaggerAssets() {
  const out = {}
  for (const [name, contentType] of Object.entries(SWAGGER_ASSET_TYPES)) {
    const filePath = path.join(SWAGGER_ASSETS_DIR, name)
    if (!fs.existsSync(filePath)) continue
    out[name] = { contentType, body: fs.readFileSync(filePath, 'utf8') }
  }
  return out
}

const SWAGGER_ASSETS = loadSwaggerAssets()

function swaggerAssetUrl(prefix, name) {
  return `${prefix}/assets/${name}`
}

function mountSwaggerAssets(engine, prefix) {
  const assetsPrefix = `${prefix}/assets`
  for (const [name, { contentType, body }] of Object.entries(SWAGGER_ASSETS)) {
    engine.route('GET', `${assetsPrefix}/${name}`, () => ({
      status: 200,
      body,
      headers: { 'content-type': contentType },
    }))
  }
}

function normalizeVersionLabel(value) {
  return String(value || '')
    .trim()
    .replace(/^\/+|\/+$/g, '')
}

function collectRouteVersions() {
  const versions = []
  let hasUnversioned = false
  for (const item of registry) {
    const version = normalizeVersionLabel(item.version_prefix)
    if (!version) {
      hasUnversioned = true
      continue
    }
    if (!versions.includes(version)) versions.push(version)
  }
  return { versions, hasUnversioned }
}

function clearRouteRegistry() {
  registry.length = 0
}

function testSwaggerConfig() {
  return {
    path: '/swagger',
    info: { title: 'fusion-framework', version: '1.0.0' },
    servers: [],
    auth: { schemes: {}, global: [], oauth: {} },
    navbar: {
      enabled: true,
      showUrlInput: false,
      showUrlInputSet: true,
      urlsSet: false,
      urls: [],
    },
    ui: {},
    pageTitle: 'Fusion API Docs',
  }
}

function openapiSpec(version = null) {
  return buildOpenApi(testSwaggerConfig(), version)
}

function routeVersions() {
  return collectRouteVersions().versions
}

function hasUnversionedRoutes() {
  return collectRouteVersions().hasUnversioned
}

function swaggerVersionUrls(prefix) {
  const { versions, hasUnversioned } = collectRouteVersions()
  const urls = versions.map((label) => ({
    url: `${prefix}/${label}/openapi.json`,
    name: label,
  }))
  if (hasUnversioned && urls.length) {
    urls.push({
      url: `${prefix}/${UNVERSIONED_SWAGGER_NAME}/openapi.json`,
      name: UNVERSIONED_SWAGGER_NAME,
    })
  }
  return urls
}

function applyVersionNavbar(swagger) {
  const autoUrls = swaggerVersionUrls(swagger.path)
  if (!swagger.navbar.urlsSet && autoUrls.length) {
    swagger.navbar.urls = autoUrls
    if (!swagger.navbar.showUrlInputSet) swagger.navbar.showUrlInput = false
  }
  return autoUrls.map((item) => item.name)
}

function routeMatchesVersion(item, filter) {
  const version = normalizeVersionLabel(item.version_prefix)
  if (filter == null) return true
  const label = normalizeVersionLabel(filter)
  if (!label || label.toLowerCase() === UNVERSIONED_SWAGGER_NAME) return !version
  return version.toLowerCase() === label.toLowerCase()
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

const OPENAPI_PERMISSIONS_SCHEME = 'FusionPermissions'

function isTemplateClass(ApiClass) {
  let current = ApiClass
  while (current && current !== Function.prototype) {
    if (current === FusionBaseTemplate || current.__fusion_template__) return true
    current = Object.getPrototypeOf(current)
  }
  return false
}

function fillOpenApiPaths(openapi, versionFilter = null) {
  const parsePathParams = (pattern) => {
    return String(pattern)
      .split('/')
      .filter((seg) => (seg.startsWith('{') && seg.endsWith('}')) || (seg.startsWith('[') && seg.endsWith(']')))
      .map((seg) => seg.slice(1, -1))
  }

  let anyPermissions = false

  for (const item of registry) {
    if (!routeMatchesVersion(item, versionFilter)) continue
    const { ApiClass, swagger: routeSwagger, requiresPermissions } = item
    if (isTemplateClass(ApiClass)) continue
    if (requiresPermissions) anyPermissions = true
    const slots = item.slots || []

    for (const slot of slots) {
      const pathParams = parsePathParams(slot.path)
      const resolvedPath = slot.path.startsWith('/') ? slot.path : `/${slot.path}`
      const routeSwaggerEntry = slot.swagger || routeSwagger

      if (!openapi.paths[resolvedPath]) openapi.paths[resolvedPath] = {}

      const methodLower = String(slot.httpMethod).toLowerCase()
      const methodUpper = methodLower.toUpperCase()
      const params = pathParams.map((name) => ({
        name,
        in: 'path',
        required: true,
        schema: { type: 'string' },
      }))

      openapi.paths[resolvedPath][methodLower] = {
        tags: routeSwaggerEntry?.tags?.length ? routeSwaggerEntry.tags : [],
        summary: routeSwaggerEntry?.title ?? `${ApiClass.name}.${slot.handlerMethod}`,
        description: routeSwaggerEntry?.description ?? '',
        deprecated: !!routeSwaggerEntry?.deprecated,
        operationId: `${ApiClass.name}_${slot.handlerMethod}`,
        parameters: params,
        responses: {
          200: { description: 'OK' },
          ...(requiresPermissions ? { 403: { description: 'Forbidden — permission check failed' } } : {}),
        },
        ...(requiresPermissions ? { security: [{ [OPENAPI_PERMISSIONS_SCHEME]: [] }] } : {}),
      }
    }
  }

  if (anyPermissions) {
    openapi.components = asObject(openapi.components)
    openapi.components.securitySchemes = {
      ...asObject(openapi.components.securitySchemes),
      [OPENAPI_PERMISSIONS_SCHEME]: {
        type: 'apiKey',
        in: 'header',
        name: 'Authorization',
        description: 'Route requires custom permission checks to pass',
      },
    }
  }
  return openapi
}

function buildOpenApi(swagger, version = null) {
  const openapi = applySwaggerOpenApi(
    {
      openapi: '3.0.3',
      info: { ...swagger.info },
      paths: {},
    },
    swagger,
  )
  const label = normalizeVersionLabel(version)
  if (label && label !== UNVERSIONED_SWAGGER_NAME) {
    openapi.info = { ...openapi.info, version: label }
  }
  return fillOpenApiPaths(openapi, version)
}

function swaggerUiHtml(swagger, openapiUrl, primaryName = null) {
  const uiOpts = { ...swagger.ui }
  delete uiOpts.presets
  delete uiOpts.plugins
  delete uiOpts.layout

  if (swagger.navbar?.urls?.length) {
    delete uiOpts.url
    uiOpts.urls = swagger.navbar.urls
    const name = primaryName || swagger.navbar.urls[0]?.name
    if (name) uiOpts['urls.primaryName'] = name
  } else {
    uiOpts.url = openapiUrl
    delete uiOpts.urls
    delete uiOpts['urls.primaryName']
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
  const versionUrls = swagger.navbar?.urls?.length > 0
  const needsStandalone = navbarEnabled || versionUrls
  const hideUrlCss =
    navbarEnabled && !showUrlInput
      ? `<style>
      .swagger-ui .topbar .download-url-wrapper input[type=text],
      .swagger-ui .topbar .download-url-wrapper .download-url-button {
        display: none !important;
      }
    </style>`
      : ''
  const standaloneScript =
    needsStandalone && SWAGGER_ASSETS['swagger-ui-standalone-preset.js']
      ? `<script src="${swaggerAssetUrl(swagger.path, 'swagger-ui-standalone-preset.js')}"></script>`
      : ''

  return `<!doctype html>
<html>
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>${title}</title>
    <link rel="stylesheet" href="${swaggerAssetUrl(swagger.path, 'swagger-ui.css')}" />
    ${hideUrlCss}
  </head>
  <body>
    <div id="swagger-ui"></div>
    <script src="${swaggerAssetUrl(swagger.path, 'swagger-ui-bundle.js')}"></script>
    ${standaloneScript}
    <script>
      window.onload = function() {
        var opts = ${uiJson};
        opts.presets = [SwaggerUIBundle.presets.apis];
        opts.plugins = [SwaggerUIBundle.plugins.DownloadUrl];
        if (${needsStandalone ? 'true' : 'false'} && typeof SwaggerUIStandalonePreset !== 'undefined') {
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
    this._middleware = []
  }

  use(middleware) {
    this._middleware.push(middleware)
  }

  mount() {
    if (this.mounted) return
    activeGlobalMiddleware = [...this._middleware]

    for (const { ApiClass, middleware: routeMiddleware = [], slots = [] } of registry) {
      for (const slot of slots) {
        const handlerMethod = slot.handlerMethod
        this.engine.route(String(slot.httpMethod).toUpperCase(), slot.path, (errOrRequest, maybeRequest) => {
          const request = nativeRequestArg(errOrRequest, maybeRequest)
          const chain = [...activeGlobalMiddleware, ...routeMiddleware]
          const handler = async (req) => {
            try {
              const instance = new ApiClass(req || emptyRequest())
              const fn = instance[handlerMethod]
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

    mountStaticFiles(this.engine, this._middleware)

    const swagger = readSwaggerSettings()
    if (swagger.enabled) {
      const prefix = swagger.path
      mountSwaggerAssets(this.engine, prefix)
      const labels = applyVersionNavbar(swagger)
      const combined = buildOpenApi(swagger)

      const htmlEnvelope = (primaryName = null) => ({
        status: 200,
        body: swaggerUiHtml(swagger, `${prefix}/openapi.json`, primaryName),
        headers: { 'content-type': 'text/html' },
      })

      this.engine.route('GET', `${prefix}/openapi.json`, () => combined)
      this.engine.route('GET', prefix, () => htmlEnvelope())
      if (prefix !== '/') {
        this.engine.route('GET', `${prefix}/`, () => htmlEnvelope())
      }

      for (const label of labels) {
        const spec = buildOpenApi(swagger, label)
        this.engine.route('GET', `${prefix}/${label}/openapi.json`, () => spec)
        this.engine.route('GET', `${prefix}/${label}`, () => htmlEnvelope(label))
        this.engine.route('GET', `${prefix}/${label}/`, () => htmlEnvelope(label))
      }
    }

    this.mounted = true
  }

  async listen(host, port, options = {}) {
    const reloadOpt =
      options && Object.prototype.hasOwnProperty.call(options, 'reload')
        ? options.reload
        : host && typeof host === 'object'
          ? host.reload
          : undefined
    // Support listen({ host, port, reload }) as well as listen(host, port, { reload })
    let h = host
    let p = port
    let reloadArg = reloadOpt
    let watchDirs = options?.watchDirs
    if (host && typeof host === 'object' && !Array.isArray(host)) {
      h = host.host
      p = host.port
      reloadArg = host.reload
      watchDirs = host.watchDirs
    }

    const snapshot = getSettings()
    const settingsReload = Boolean(snapshot.get('reload', false))
    const shouldReload =
      reloadArg === undefined || reloadArg === null ? settingsReload : Boolean(reloadArg)

    if (shouldReload && process.env.FUSION_RELOAD_CHILD !== '1') {
      await runWithReloader({ watchDirs })
      return
    }

    this.mount()
    h = h ?? snapshot.host
    p = p ?? snapshot.port
    if (snapshot.debug || shouldReload) {
      const mode = shouldReload ? ' (reload)' : ''
      console.log(`fusion listening on http://${h}:${p}${mode}`)
    }
    await this.engine.listen(h, Number(p))
  }
}

const RELOAD_SKIP_DIRS = new Set([
  '.git',
  '.hg',
  'node_modules',
  'target',
  '.venv',
  'venv',
  '__pycache__',
  'bin',
  'obj',
  'dist',
  'build',
])

const RELOAD_EXTENSIONS = new Set([
  '.js',
  '.mjs',
  '.cjs',
  '.ts',
  '.json',
  '.html',
  '.tera',
  '.py',
  '.cs',
])

function collectWatchedFiles(roots) {
  const files = []
  const walk = (dir) => {
    let entries
    try {
      entries = fs.readdirSync(dir, { withFileTypes: true })
    } catch {
      return
    }
    for (const entry of entries) {
      if (entry.name.startsWith('.') && entry.name !== '.') continue
      const full = path.join(dir, entry.name)
      if (entry.isDirectory()) {
        if (RELOAD_SKIP_DIRS.has(entry.name)) continue
        walk(full)
      } else if (RELOAD_EXTENSIONS.has(path.extname(entry.name).toLowerCase())) {
        files.push(full)
      }
    }
  }
  for (const root of roots) {
    const resolved = path.resolve(root)
    try {
      const st = fs.statSync(resolved)
      if (st.isFile()) files.push(resolved)
      else if (st.isDirectory()) walk(resolved)
    } catch {
      /* missing root */
    }
  }
  return files
}

function snapshotMtimes(files) {
  const map = new Map()
  for (const file of files) {
    try {
      map.set(file, fs.statSync(file).mtimeMs)
    } catch {
      /* ignore */
    }
  }
  return map
}

async function runWithReloader({ watchDirs } = {}) {
  const roots = watchDirs?.length ? watchDirs : [process.cwd()]
  console.log(`fusion: reload enabled (watching ${roots.join(', ')})`)

  let child = null
  const spawnChild = () => {
    const env = { ...process.env, FUSION_RELOAD_CHILD: '1' }
    child = spawn(process.execPath, process.argv.slice(1), {
      env,
      stdio: 'inherit',
    })
    return child
  }

  const stopChild = () =>
    new Promise((resolve) => {
      if (!child || child.exitCode !== null) {
        child = null
        resolve()
        return
      }
      child.once('exit', () => {
        child = null
        resolve()
      })
      child.kill('SIGTERM')
      setTimeout(() => {
        if (child) child.kill('SIGKILL')
      }, 5000)
    })

  const shutdown = async () => {
    await stopChild()
    process.exit(0)
  }
  process.on('SIGINT', shutdown)
  process.on('SIGTERM', shutdown)

  let mtimes = snapshotMtimes(collectWatchedFiles(roots))
  spawnChild()

  // eslint-disable-next-line no-constant-condition
  while (true) {
    await new Promise((r) => setTimeout(r, 500))
    if (child && child.exitCode !== null) {
      console.log(`fusion: child exited (${child.exitCode}); restarting…`)
      await new Promise((r) => setTimeout(r, 300))
      spawnChild()
      mtimes = snapshotMtimes(collectWatchedFiles(roots))
      continue
    }
    const files = collectWatchedFiles(roots)
    const next = snapshotMtimes(files)
    let changed = null
    for (const [file, mtime] of next) {
      const prev = mtimes.get(file)
      if (prev === undefined || mtime > prev) {
        changed = file
        break
      }
    }
    if (!changed) {
      for (const file of mtimes.keys()) {
        if (!next.has(file)) {
          changed = file
          break
        }
      }
    }
    if (!changed) continue
    let label = changed
    try {
      label = path.relative(process.cwd(), changed) || changed
    } catch {
      /* keep absolute */
    }
    console.log(`fusion: change detected (${label}); reloading…`)
    await stopChild()
    spawnChild()
    mtimes = snapshotMtimes(collectWatchedFiles(roots))
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
  await app.listen({
    reload: options && Object.prototype.hasOwnProperty.call(options, 'reload')
      ? options.reload
      : undefined,
    watchDirs: options?.watchDirs,
  })
  return app
}

function coerceParam(raw, kind = 'auto') {
  return native.coerceParamJs(String(raw), kind)
}

function pathToFileUrl(filePath) {
  const resolved = path.resolve(filePath)
  return require('url').pathToFileURL(resolved).href
}

function parsePagination(query, { defaultPageSize = 20, maxPageSize = 100 } = {}) {
  try {
    return native.parsePagination(query, defaultPageSize, maxPageSize)
  } catch (err) {
    throw new HTTPException(400, err?.message || 'invalid pagination')
  }
}

function paginatedBody(items, total, params) {
  return native.paginatedBody(items, total, params)
}

/** Process-wide application cache (default driver: moka). */
const cache = {
  _ready: false,
  _ensure() {
    if (this._ready) return
    try {
      // Use the native Settings singleton (not getSettings()'s plain object).
      settings.ensureLoaded([process.cwd()])
      native.cacheConfigure(settings)
      this._ready = true
    } catch {
      native.cacheConfigureDriver('moka', null, null)
      this._ready = true
    }
  },
  configure(settingsInstance) {
    const s = settingsInstance || settings
    if (s && typeof s.ensureLoaded === 'function') {
      s.ensureLoaded([process.cwd()])
    }
    native.cacheConfigure(s)
    this._ready = true
  },
  configureDriver(driver = 'moka', { maxCapacity, defaultTtl } = {}) {
    native.cacheConfigureDriver(driver, maxCapacity ?? null, defaultTtl ?? null)
    this._ready = true
  },
  set(key, value, ttl = null) {
    this._ensure()
    native.cacheSet(key, value, ttl)
  },
  get(key) {
    this._ensure()
    return native.cacheGet(key)
  },
  delete(key) {
    this._ensure()
    return native.cacheDelete(key)
  },
  exists(key) {
    this._ensure()
    return native.cacheExists(key)
  },
  getOrSet(key, defaultValue, ttl = null) {
    this._ensure()
    if (native.cacheExists(key)) return native.cacheGet(key)
    const value = typeof defaultValue === 'function' ? defaultValue() : defaultValue
    return native.cacheGetOrSet(key, value, ttl)
  },
  deleteOrSet(key, value, ttl = null) {
    this._ensure()
    return native.cacheDeleteOrSet(key, value, ttl)
  },
  existsOrSet(key, value, ttl = null) {
    this._ensure()
    return native.cacheExistsOrSet(key, value, ttl)
  },
  clear() {
    this._ensure()
    native.cacheClear()
  },
  driver() {
    this._ensure()
    return native.cacheDriver()
  },
  reset() {
    native.cacheReset()
    this._ready = false
  },

  /** Async set (Promise). */
  async aset(key, value, ttl = null) {
    this.set(key, value, ttl)
  },
  async aget(key) {
    return this.get(key)
  },
  async adelete(key) {
    return this.delete(key)
  },
  async aexists(key) {
    return this.exists(key)
  },
  async agetOrSet(key, defaultValue, ttl = null) {
    this._ensure()
    if (native.cacheExists(key)) return native.cacheGet(key)
    let value = typeof defaultValue === 'function' ? defaultValue() : defaultValue
    if (value && typeof value.then === 'function') value = await value
    return native.cacheGetOrSet(key, value, ttl)
  },
  async adeleteOrSet(key, value, ttl = null) {
    return this.deleteOrSet(key, value, ttl)
  },
  async aexistsOrSet(key, value, ttl = null) {
    return this.existsOrSet(key, value, ttl)
  },
  async aclear() {
    this.clear()
  },
}

const route = router

module.exports = {
  App: NativeApp,
  Settings: NativeSettings,
  FusionApp,
  FusionBaseApi,
  FusionBaseTemplate,
  HTTPException,
  router,
  route,
  httpGet,
  httpPost,
  httpPut,
  httpPatch,
  httpDelete,
  httpHead,
  httpOptions,
  apiResourceName,
  resolveRoutePath,
  resolveHandlerRoute,
  apiActionName,
  configure,
  getSettings,
  settings,
  status,
  header,
  HTTP_METHODS,
  run,
  bearerJwt,
  requireRoles,
  requirePermissions,
  frameworkHeaders,
  securityHeaders,
  cors,
  cacheHeaders,
  requestId,
  staticFiles,
  runMiddlewareChain,
  coerceParam,
  parsePagination,
  paginatedBody,
  cache,
  renderTemplate,
  clearRouteRegistry,
  openapiSpec,
  routeVersions,
  hasUnversionedRoutes,
  getHttpMethods: () => HTTP_METHODS,
  apiResourceNameJs: native.apiResourceNameJs,
  resolveRoutePathJs: native.resolveRoutePathJs,
  coerceParamJs: native.coerceParamJs,
}
