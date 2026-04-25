'use strict'

const native = require('./index.js')

function defineConfig(config) {
  return config
}

module.exports = {
  compile: native.compile,
  build: native.build,
  defineConfig,
}
