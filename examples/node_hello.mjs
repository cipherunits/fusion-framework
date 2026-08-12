import { createRequire } from 'module'
const require = createRequire(import.meta.url)
const { FusionBaseApi, router } = require('../crates/fusion-node/index.js')

// /api/[module]/{id} + MyFirstModule → /api/myfirst/{id}
export const MyFirstModule = router('/api/[module]/{id}')(
  class MyFirstModule extends FusionBaseApi {
    get() {
      return { status: 200, body: `hello from node, id=${this.params.id}` }
    }

    post() {
      return { status: 200, body: `hello from node, id=${this.params.id}` }
    }

    put() {
      return { status: 200, body: `hello from node, id=${this.params.id}` }
    }

    delete() {
      return { status: 200, body: `hello from node, id=${this.params.id}` }
    }

    patch() {
      return { status: 200, body: `hello from node, id=${this.params.id}` }
    }
  },
)

export const RootApi = router('/')(
  class RootApi extends FusionBaseApi {
    get() {
      return this.response('fusion class-based api')
    }
  },
)
