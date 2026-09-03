export class App {
  constructor()
  route(method: string, path: string, handler: (req: FusionRequest) => FusionResponse | string): void
  listen(host: string, port: number): Promise<void>
}

export class Settings {
  constructor()
  loadJson(path?: string | null, env?: string | null, extraRoots?: string[]): void
  ensureLoaded(extraRoots?: string[]): void
  merge(values: Record<string, unknown>): void
  get(key: string, defaultValue?: unknown): unknown
  readonly host: string
  readonly port: number
  readonly debug: boolean
  readonly env: string
}

export class FusionBaseApi {
  request: FusionRequest
  constructor(request: FusionRequest)
  readonly method: string
  readonly path: string
  readonly body: string
  readonly headers: Record<string, string>
  readonly params: Record<string, string>
  readonly query: Record<string, string>
  readonly state: Record<string, unknown>
  wantsJson(): boolean
  response(body?: unknown, status?: number, headers?: Record<string, string>): FusionResponse
}

export class FusionBaseTemplate extends FusionBaseApi {
  static template: string
  static templateAddress: string
  static templatesDir: string
  context(): Record<string, unknown>
  get(): FusionResponse
  templateName(): string
  templatesRoot(): string
  render(options?: {
    status?: number
    headers?: Record<string, string>
    context?: Record<string, unknown>
    templateName?: string
  }): FusionResponse
}

export function renderTemplate(
  templateName: string,
  context?: Record<string, unknown>,
  templatesRoot?: string | null,
): string

export class HTTPException extends Error {
  status: number
  detail: unknown
  headers: Record<string, string>
  constructor(status: number, detail?: unknown, headers?: Record<string, string>)
  toResponse(): FusionResponse
}

export class FusionApp {
  constructor(settings?: Partial<FusionSettings>)
  use(middleware: FusionMiddleware): void
  mount(): void
  listen(
    host?: string | {
      host?: string
      port?: number
      reload?: boolean
      watchDirs?: string[]
    },
    port?: number,
    options?: { reload?: boolean; watchDirs?: string[] },
  ): Promise<void>
}

export type RouteOptions = {
  tags?: string[]
  desc?: string
  title?: string
  version?: string
  deprecated?: boolean
  middleware?: FusionMiddleware[]
  permissions?: Array<(request: FusionRequest) => boolean>
}

export function router(path: string, options?: RouteOptions): <T>(ApiClass: T) => T
/** Alias of `router`. */
export function route(path: string, options?: RouteOptions): <T>(ApiClass: T) => T

export function bearerJwt(options?: {
  stateKey?: string
  header?: string
  verify?: (token: string) => Record<string, unknown> | null
}): FusionMiddleware

export function requirePermissions(
  ...checks: Array<(request: FusionRequest) => boolean>
): FusionMiddleware

export function requireRoles(...roles: string[]): FusionMiddleware
export function requireRoles(options: {
  roles: string[]
  claim?: string
  stateKey?: string
}): FusionMiddleware

export function runMiddlewareChain(
  request: FusionRequest,
  middlewares: FusionMiddleware[],
  handler: (request: FusionRequest) => unknown | Promise<unknown>,
): Promise<unknown>

export function apiResourceName(cls: { name: string } | string): string
export function resolveRoutePath(path: string, cls: { name: string }): string
export function configure(settings: Record<string, unknown>): FusionSettings
export function getSettings(): FusionSettings
export function run(
  options?: string | { settingsModule?: string; middleware?: FusionMiddleware[] },
): Promise<FusionApp>
export function coerceParam(raw: string, kind?: string): unknown
export function getHttpMethods(): string[]
export function apiResourceNameJs(className: string): string
export function resolveRoutePathJs(template: string, className: string): string
export function coerceParamJs(raw: string, kind?: string): unknown
export function prefersJsonJs(accept?: string | null, formatQuery?: string | null): boolean

export const settings: Settings
export const status: Record<string, number>
export const header: HeaderModule
export const HTTP_METHODS: string[]

export interface HeaderModule {
  [name: string]: string | ((...args: any[]) => Record<string, string>)
  CONTENT_TYPE: string
  CONTENT_DISPOSITION: string
  LOCATION: string
  AUTHORIZATION: string
  APPLICATION_JSON: string
  APPLICATION_OCTET_STREAM: string
  APPLICATION_PDF: string
  attachment(filename: string): Record<string, string>
  inline(filename?: string | null): Record<string, string>
  contentType(mediaType: string, charset?: string | null): Record<string, string>
  location(url: string): Record<string, string>
  cacheControl(value: string): Record<string, string>
  download(filename: string, mediaType?: string | null): Record<string, string>
  fingerprint(): Record<string, string>
}

export interface FusionSettings {
  host: string
  port: number
  debug: boolean
  env?: string
}

export interface FusionRequest {
  method: string
  path: string
  body: string
  headers: Record<string, string>
  params: Record<string, string>
  query: Record<string, string>
  state?: Record<string, unknown>
}

export type FusionMiddleware = (
  request: FusionRequest,
  callNext: (request: FusionRequest) => unknown | Promise<unknown>,
) => unknown | Promise<unknown>

export function frameworkHeaders(): FusionMiddleware

export function securityHeaders(options?: {
  contentTypeOptions?: string
  frameOptions?: string
  referrerPolicy?: string
  permissionsPolicy?: string
  coop?: string
  corp?: string
  csp?: string
  hsts?: string
}): FusionMiddleware

export function cors(options?: {
  allowOrigins?: string | string[]
  allowMethods?: string[]
  allowHeaders?: string[]
  exposeHeaders?: string[]
  allowCredentials?: boolean
  maxAge?: number
}): FusionMiddleware

export function cacheHeaders(options?: { default?: string; value?: string }): FusionMiddleware

export function requestId(options?: {
  header?: string
  incoming?: boolean
}): FusionMiddleware

/** Serve files from `root` under URL `prefix` (WhiteNoise-style). */
export function staticFiles(options?: {
  root?: string
  prefix?: string
  maxAge?: number | null
  fallthrough?: boolean
}): FusionMiddleware

export type FusionResponse =
  | string
  | {
      status?: number
      body?: unknown
      headers?: Record<string, string>
    }
