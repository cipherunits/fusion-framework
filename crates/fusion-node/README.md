# Fusion Framework (Node.js)

Class-based HTTP APIs on a shared Rust core (`fusion-core`).

Handlers use `this.params` / `this.query` / `this.body` / `this.state` (no signature param injection — that is Python-only DX).

## Links

- **Docs:** [fusion.cipherunit.xyz](https://fusion.cipherunit.xyz/)
- **GitHub:** [cipherunits/fusion-framework](https://github.com/cipherunits/fusion-framework)
- **CLI:** [cipherunits/fusion-tool](https://github.com/cipherunits/fusion-tool)

## Install

```bash
cd crates/fusion-node
npm install
npm run build:debug
```

Or after publish:

```bash
npm i fusion-framework
```

## Quick start

```js
import { FusionBaseApi, route, status, FusionApp, getSettings, settings } from 'fusion-framework'

export const ItemModule = route('/api/[module]/{id}')(
  class ItemModule extends FusionBaseApi {
    get() {
      return this.response({ id: this.params.id }, status.HTTP_SUCCESS)
    }
  },
)

const MIDDLEWARE = [] // optional — framework has no defaults

settings.ensureLoaded()
const app = new FusionApp(getSettings())
for (const mw of MIDDLEWARE) app.use(mw)
await app.listen()
```

## Middleware

```js
import { bearerJwt, requireRoles, route } from 'fusion-framework'

const MIDDLEWARE = [bearerJwt()] // or bearerJwt({ verify })

route('/api/admin', { roles: ['admin', 'super_admin'] })(
  class AdminModule extends FusionBaseApi {
    get() {
      return this.response({ user: this.state.jwt?.sub })
    }
  },
)
```

Sync or async: `(request, callNext) => …` / `async (request, callNext) => await callNext(request)`.

## Status codes

```js
import { status } from 'fusion-framework'
status.HTTP_SUCCESS // 200
status.HTTP_404_NOT_FOUND
```

## License

MIT
