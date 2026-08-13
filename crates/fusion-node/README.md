# Fusion Framework (Node.js)

Class-based HTTP APIs on a shared Rust core (`fusion-core`).

Handlers use `this.params` / `this.query` / `this.body` / `this.state` (no signature param injection — that is Python-only).

## Install

```bash
npm i fusion-framework
```

From this repo:

```bash
cd crates/fusion-node && npm install && npm run build:debug
```

Scaffold with [Fusion Tool](https://github.com/cipherunits/fusion-tool):

```bash
fusion init --lang typescript --name my-app
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

const MIDDLEWARE = [] // optional — no defaults

settings.ensureLoaded()
const app = new FusionApp(getSettings())
for (const mw of MIDDLEWARE) app.use(mw)
await app.listen()
```

## Docs

Full guides (router, config, middleware):  
**https://fusion.cipherunit.xyz/en/docs/typescript/v1**

- Site: [fusion.cipherunit.xyz](https://fusion.cipherunit.xyz/)
- GitHub: [cipherunits/fusion-framework](https://github.com/cipherunits/fusion-framework)

## License

BSD 3-Clause
