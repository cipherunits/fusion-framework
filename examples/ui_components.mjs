/**
 * Render Fusion Tera UI components (button, badge, table + page_size).
 *
 *   node examples/ui_components.mjs
 */
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { renderTemplate } from 'fusion-framework'

const root = join(dirname(fileURLToPath(import.meta.url)), 'ui_components_assets')

const html = renderTemplate(
  'page.html',
  {
    headers: ['Route', 'Method'],
    rows: [
      ['/v1/api/product', 'GET'],
      ['/swagger', 'GET'],
      ['/__fusion/cache', 'GET'],
      ['/health', 'GET'],
    ],
  },
  root,
)

console.log(html)
