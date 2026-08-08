import { createRequire } from 'module'
const require = createRequire(import.meta.url)
const { FusionBaseApi, router } = require('../crates/fusion-node/index.js')

// /api/[module]/{id} + MyFirstModule → /api/myfirst/{id}
export const MyFirstModule = router('/api/[module]/{id}')(
  class MyFirstModule extends FusionBaseApi {
    get(id) {
      return { status: 200, body: `hello from node, id=${id}` }
    }

    post(id) {
      return { status: 200, body: `hello from node, id=${id}` }
    }

    put(id) {
      return { status: 200, body: `hello from node, id=${id}` }
    }

    delete(id) {
      return { status: 200, body: `hello from node, id=${id}` }
    }

    patch(id) {
      return { status: 200, body: `hello from node, id=${id}` }
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
