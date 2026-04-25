import { createRequire } from 'node:module'
import { readFileSync, writeFileSync, unlinkSync } from 'node:fs'

const require = createRequire(import.meta.url)
const native = require('./index.js')

class SchemaBuilder {
  constructor(descriptor) {
    Object.assign(this, descriptor)
  }
  toJSON() {
    const out = {}
    for (const k of Object.keys(this)) out[k] = this[k]
    return out
  }
  optional() { return new SchemaBuilder({ kind: 'optional', inner: this.toJSON() }) }
  nullable() { return new SchemaBuilder({ kind: 'nullable', inner: this.toJSON() }) }
  default(value) { return new SchemaBuilder({ kind: 'default', inner: this.toJSON(), fallback: value }) }
  min(n) { return new SchemaBuilder({ ...this.toJSON(), min: n }) }
  max(n) { return new SchemaBuilder({ ...this.toJSON(), max: n }) }
  length(n) { return new SchemaBuilder({ ...this.toJSON(), length: n }) }
  regex(p) { return new SchemaBuilder({ ...this.toJSON(), regex: p }) }
  int() { return new SchemaBuilder({ ...this.toJSON(), int: true }) }
  by(bucket) { return new SchemaBuilder({ ...this.toJSON(), bucket }) }
  reserved(list) { return new SchemaBuilder({ ...this.toJSON(), reserved: list }) }
  passthrough() { return new SchemaBuilder({ ...this.toJSON(), passthrough: true }) }
}

const sb = (d) => new SchemaBuilder(d)

export const s = {
  string: () => sb({ kind: 'string' }),
  number: () => sb({ kind: 'number' }),
  boolean: () => sb({ kind: 'boolean' }),
  array: (item) => sb({ kind: 'array', item }),
  object: (fields) => sb({ kind: 'object', fields }),
  enum: (variants) => sb({ kind: 'enum', variants }),
  literal: (expected) => sb({ kind: 'literal', expected }),
  union: (variants) => sb({ kind: 'union', variants }),
  raw: () => sb({ kind: 'raw' }),
  markdown: () => sb({ kind: 'markdown' }),
  mdx: () => sb({ kind: 'mdx' }),
  toc: () => sb({ kind: 'toc' }),
  metadata: () => sb({ kind: 'metadata' }),
  excerpt: (opts = {}) => sb({ kind: 'excerpt', ...opts }),
  path: (opts = {}) => sb({ kind: 'path', ...opts }),
  slug: (bucket, reserved) => sb({ kind: 'slug', bucket, reserved }),
  unique: (bucket) => sb({ kind: 'unique', bucket }),
  isodate: () => sb({ kind: 'isodate' }),
}

export const defineConfig = (config) => config
export const defineCollection = (c) => c
export const defineLoader = (l) => l
export const defineSchema = (sch) => sch

function adaptToBuildInput(input) {
  if (Array.isArray(input?.collections)) return input
  const root = input.root ?? '.'
  const outputDir = input.output?.data ?? '.gentleduck'
  const collections = Object.entries(input.collections ?? {}).map(([key, c]) => ({
    name: c.name ?? key,
    pattern: Array.isArray(c.pattern) ? c.pattern[0] : c.pattern,
    baseDir: c.baseDir ?? root,
    schema: c.schema,
    single: c.single,
  }))
  return {
    outputDir,
    collections,
    root,
    strict: input.strict,
    clean: input.output?.clean,
    outputAssets: input.output?.assets,
    outputBase: input.output?.base,
    outputName: input.output?.name,
    outputFormat: input.output?.format,
  }
}

export const compile = native.compile
export const compileMany = native.compileMany

export async function build(input) {
  const report = native.build(adaptToBuildInput(input))
  if (input?.prepare || input?.complete) {
    const data = {}
    for (const c of report.collections) {
      data[c.name] = JSON.parse(readFileSync(c.outputPath, 'utf8'))
    }
    if (input.prepare) {
      const ret = await input.prepare(data, { config: input })
      if (ret === false) {
        for (const c of report.collections) try { unlinkSync(c.outputPath) } catch {}
        return report
      }
      for (const c of report.collections) {
        writeFileSync(c.outputPath, JSON.stringify(data[c.name], null, 2))
      }
    }
    if (input.complete) await input.complete(data, { config: input })
  }
  return report
}

export default { compile, compileMany, build, defineConfig, defineCollection, defineLoader, defineSchema, s }
