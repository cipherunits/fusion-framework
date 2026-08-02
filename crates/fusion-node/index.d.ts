export class App {
  constructor()
  route(method: string, path: string, handler: (req: FusionRequest) => FusionResponse | string): void
  listen(host: string, port: number): Promise<void>
}

export class FusionBaseApi {
  request: FusionRequest
  constructor(request: FusionRequest)
  readonly method: string
  readonly path: string
  readonly body: string
  readonly headers: Record<string, string>
  readonly params: Record<string, string>
  ok(body?: string, status?: number, headers?: Record<string, string>): FusionResponse
  json(data: unknown, status?: number): FusionResponse
}

export class FusionApp {
  constructor(settings?: Partial<FusionSettings>)
  mount(): void
  listen(host?: string, port?: number): Promise<void>
}

export function router(path: string): <T>(ApiClass: T) => T
export function apiResourceName(cls: { name: string } | string): string
export function resolveRoutePath(path: string, cls: { name: string }): string
export function configure(settings: Partial<FusionSettings>): FusionSettings
export function getSettings(): FusionSettings
export function run(settingsModulePath?: string): Promise<void>

export interface FusionSettings {
  host: string
  port: number
  debug: boolean
}

export interface FusionRequest {
  method: string
  path: string
  body: string
  headers: Record<string, string>
  params: Record<string, string>
}

export type FusionResponse =
  | string
  | {
      status?: number
      body?: string
      headers?: Record<string, string>
    }
