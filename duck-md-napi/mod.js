'use strict'

const native = require('./index.js')

/**
 * Identity helper for typed config objects.
 * Mirrors velite's `defineConfig` ergonomic.
 */
function defineConfig(config) {
  return config
}

module.exports = {
  compile: native.compile,
  build: native.build,
  defineConfig,
}
