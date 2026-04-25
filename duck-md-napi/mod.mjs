import { createRequire } from 'node:module'

const require = createRequire(import.meta.url)
const native = require('./index.js')

export const compile = native.compile
export const build = native.build
export function defineConfig(config) {
  return config
}

export default { compile, build, defineConfig }
