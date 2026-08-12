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

  get query() {
    return this.request.query || {}
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

function router(routePath, options = {}) {
  return function decorate(ApiClass) {
    const resolvedBase = resolveRoutePath(routePath, ApiClass)

    const v = (options.version ?? '').toString().trim()
    const resolved =
      v.length > 0 ? `${v}/${resolvedBase.replace(/^\/+/, '')}` : resolvedBase

    ApiClass.__fusion_path__ = resolved
    ApiClass.__fusion_path_template__ = routePath

    registry.push({
      path: resolved,
      ApiClass,
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
  }

  mount() {
    if (this.mounted) return
    for (const { path: routePath, ApiClass } of registry) {
      for (const methodName of HTTP_METHODS) {
        if (!definesMethod(ApiClass, methodName)) continue
        this.engine.route(methodName.toUpperCase(), routePath, async (request) => {
          try {
            const instance = new ApiClass(request)
            const fn = instance[methodName]
            // No-arg handler contract: use `this.params`, `this.query`, `this.body`.
            return await Promise.resolve(fn.call(instance))
          } catch (err) {
            if (err instanceof HTTPException) return err.toResponse()
            throw err
          }
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
  HTTPException,
  router,
  apiResourceName,
  resolveRoutePath,
  configure,
  getSettings,
  settings,
  HTTP_METHODS,
  run,
}
