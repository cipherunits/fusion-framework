import { createRequire } from 'module'
const require = createRequire(import.meta.url)
const { FusionBaseApi, route, status } = require('../crates/fusion-node/index.js')

// /api/[module]/{id} + MyFirstModule → /api/myfirst/{id}
export const MyFirstModule = route('/api/[module]/{id}')(
  class MyFirstModule extends FusionBaseApi {
    get() {
      return this.response(`hello from node, id=${this.params.id}`, status.HTTP_SUCCESS)
    }

    post() {
      return this.response(`hello from node, id=${this.params.id}`, status.HTTP_SUCCESS)
    }

    put() {
      return this.response(`hello from node, id=${this.params.id}`, status.HTTP_SUCCESS)
    }

    delete() {
      return this.response(`hello from node, id=${this.params.id}`, status.HTTP_SUCCESS)
    }

    patch() {
      return this.response(`hello from node, id=${this.params.id}`, status.HTTP_SUCCESS)
    }
  },
)

export const RootApi = route('/')(
  class RootApi extends FusionBaseApi {
    get() {
      return this.response('fusion class-based api')
    }

    async post() {
      return this.response('async post ok', status.HTTP_SUCCESS)
    }
  },
)
