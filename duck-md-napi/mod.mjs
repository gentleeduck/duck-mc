import { createRequire } from 'node:module'
import { readFileSync, writeFileSync, unlinkSync } from 'node:fs'

const require = createRequire(import.meta.url)
const native = require('./index.js')

const __callbacks = new WeakMap()
let __cbId = 0
const __cbRegistry = new Map()

function registerCallback(fn) {
  const id = ++__cbId
  __cbRegistry.set(id, fn)
  return id
}

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
  transform(fn) {
    return new SchemaBuilder({ kind: 'transform', inner: this.toJSON(), __callbackId: registerCallback(fn) })
  }
  refine(fn, message) {
    return new SchemaBuilder({ kind: 'refine', inner: this.toJSON(), __callbackId: registerCallback(fn), __message: message })
  }
}

function collectCallbacks(descriptor, path = []) {
  const found = []
  if (!descriptor || typeof descriptor !== 'object') return found
  if (descriptor.kind === 'transform' && descriptor.__callbackId) {
    found.push({ path: [...path], kind: 'transform', fn: __cbRegistry.get(descriptor.__callbackId) })
  }
  if (descriptor.kind === 'refine' && descriptor.__callbackId) {
    found.push({ path: [...path], kind: 'refine', fn: __cbRegistry.get(descriptor.__callbackId), message: descriptor.__message })
  }
  if (descriptor.inner) found.push(...collectCallbacks(descriptor.inner, path))
  if (descriptor.kind === 'object' && descriptor.fields) {
    for (const [k, v] of Object.entries(descriptor.fields)) {
      found.push(...collectCallbacks(v, [...path, k]))
    }
  }
  if (descriptor.kind === 'array' && descriptor.item) {
    found.push(...collectCallbacks(descriptor.item, [...path, '*']))
  }
  return found
}

function applyCallbacks(record, callbacks, errors, file) {
  for (const cb of callbacks) {
    const targets = walkPath(record, cb.path)
    for (const { parent, key } of targets) {
      const v = parent[key]
      if (cb.kind === 'transform') {
        try { parent[key] = cb.fn(v) } catch (e) {
          errors.push({ file, message: `${cb.path.join('.')}: transform threw: ${e.message ?? e}` })
        }
      } else if (cb.kind === 'refine') {
        let ok = false
        try { ok = !!cb.fn(v) } catch (e) {
          errors.push({ file, message: `${cb.path.join('.')}: refine threw: ${e.message ?? e}` })
          continue
        }
        if (!ok) errors.push({ file, message: `${cb.path.join('.')}: ${cb.message ?? 'failed refinement'}` })
      }
    }
  }
}

function walkPath(obj, path) {
  if (path.length === 0) return [{ parent: { _: obj }, key: '_' }]
  if (path[0] === '*') {
    if (!Array.isArray(obj)) return []
    return obj.flatMap((_, i) => walkPath(obj[i], path.slice(1)).map(t => t))
  }
  const key = path[0]
  if (obj == null || typeof obj !== 'object' || !(key in obj)) return []
  if (path.length === 1) return [{ parent: obj, key }]
  return walkPath(obj[key], path.slice(1))
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
    markdownRemarkPlugins: input.markdown?.remarkPlugins,
    markdownRehypePlugins: input.markdown?.rehypePlugins,
    mdxRemarkPlugins: input.mdx?.remarkPlugins,
    mdxRehypePlugins: input.mdx?.rehypePlugins,
  }
}

export { collectCallbacks, applyCallbacks }
export const compile = native.compile
export const compileMany = native.compileMany

export async function build(input) {
  const collectionCallbacks = new Map()
  if (input?.collections) {
    for (const [key, c] of Object.entries(input.collections)) {
      if (c?.schema) {
        const cbs = collectCallbacks(c.schema.toJSON ? c.schema.toJSON() : c.schema)
        if (cbs.length) collectionCallbacks.set(c.name ?? key, cbs)
      }
    }
  }

  const report = native.build(adaptToBuildInput(input))

  const needPostprocess = collectionCallbacks.size > 0 || input?.prepare || input?.complete
  if (needPostprocess) {
    const data = {}
    for (const c of report.collections) {
      data[c.name] = JSON.parse(readFileSync(c.outputPath, 'utf8'))
    }

    if (collectionCallbacks.size > 0) {
      for (const c of report.collections) {
        const cbs = collectionCallbacks.get(c.name)
        if (!cbs) continue
        const records = Array.isArray(data[c.name]) ? data[c.name] : [data[c.name]]
        for (const record of records) {
          applyCallbacks(record, cbs, report.errors, c.outputPath)
        }
      }
    }

    if (input.prepare) {
      const ret = await input.prepare(data, { config: input })
      if (ret === false) {
        for (const c of report.collections) try { unlinkSync(c.outputPath) } catch {}
        return report
      }
    }

    for (const c of report.collections) {
      writeFileSync(c.outputPath, JSON.stringify(data[c.name], null, 2))
    }

    if (input.complete) await input.complete(data, { config: input })
  }
  return report
}

export default { compile, compileMany, build, defineConfig, defineCollection, defineLoader, defineSchema, s }
