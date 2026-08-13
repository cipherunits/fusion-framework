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
  response(body?: unknown, status?: number, headers?: Record<string, string>): FusionResponse
}

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
  listen(host?: string, port?: number): Promise<void>
}

export function router(
  path: string,
  options?: {
    tags?: string[]
    desc?: string
    title?: string
    version?: string
    deprecated?: boolean
    middleware?: FusionMiddleware[]
    roles?: string[]
  }
): <T>(ApiClass: T) => T
export function bearerJwt(options?: { stateKey?: string; header?: string }): FusionMiddleware
export function requireRoles(...roles: string[]): FusionMiddleware
export function runMiddlewareChain(
  request: FusionRequest,
  middlewares: FusionMiddleware[],
  handler: (request: FusionRequest) => unknown | Promise<unknown>,
): Promise<unknown>
export function apiResourceName(cls: { name: string } | string): string
export function resolveRoutePath(path: string, cls: { name: string }): string
export function configure(settings: Record<string, unknown>): FusionSettings
export function getSettings(): FusionSettings
export function run(settingsModulePath?: string): Promise<void>
export function getHttpMethods(): string[]
export function apiResourceNameJs(className: string): string
export function resolveRoutePathJs(template: string, className: string): string
export function coerceParamJs(raw: string, kind?: string): unknown

export const settings: Settings
export const HTTP_METHODS: string[]

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

export type FusionResponse =
  | string
  | {
      status?: number
      body?: unknown
      headers?: Record<string, string>
    }
