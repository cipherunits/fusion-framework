const path = require('path')

/** Resolve the local fusion-framework package from the monorepo. */
module.exports = require(path.resolve(__dirname, '../../../crates/fusion-node'))
