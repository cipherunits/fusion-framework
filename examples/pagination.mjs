/**
 * Paginated list API example.
 *
 * GET /v1/api/product?page=2&page_size=10
 */

import { createRequire } from 'module'

const require = createRequire(import.meta.url)
const { FusionBaseApi, route, status } = require('../crates/fusion-node/index.js')

const ALL_PRODUCTS = Array.from({ length: 50 }, (_, i) => ({
  id: i + 1,
  name: `product-${i + 1}`,
}))

export const ProductModule = route('/api/[module]', { tags: ['products'], version: 'v1' })(
  class ProductModule extends FusionBaseApi {
    get() {
      const params = this.pagination({
        page: Number(this.query.page || 1),
        pageSize: Number(this.query.page_size || this.query.per_page || this.query.limit || 20),
      })
      const start = params.offset
      const end = start + params.page_size
      const items = ALL_PRODUCTS.slice(start, end)
      return this.paginated(items, ALL_PRODUCTS.length, params, { status: status.HTTP_SUCCESS })
    }
  },
)
