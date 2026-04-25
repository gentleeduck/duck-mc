import { createRequire } from 'node:module'
import { readFileSync, writeFileSync, unlinkSync } from 'node:fs'

const require = createRequire(import.meta.url)
const native = require('./index.js')

export interface TocItem {
  title: string
  url: string
  items: TocItem[]
}

export interface Metadata {
  readingTime: number
  wordCount: number
}

export interface CompileOutput {
  body: string
  content: string
  html: string
  excerpt: string
  metadata: Metadata
  toc: TocItem[]
  frontmatter: unknown
  frontmatterRaw: string
  imports: string[]
  exports: string[]
}

export type SchemaKind =
  | 'string' | 'number' | 'boolean'
  | 'array' | 'object' | 'record' | 'tuple' | 'intersection'
  | 'enum' | 'literal' | 'union' | 'discriminatedUnion'
  | 'optional' | 'nullable' | 'default' | 'transform' | 'refine' | 'superRefine'
  | 'coerce.string' | 'coerce.number' | 'coerce.boolean' | 'coerce.date'
  | 'raw' | 'markdown' | 'mdx' | 'toc' | 'metadata' | 'excerpt'
  | 'path' | 'slug' | 'unique' | 'isodate' | 'file' | 'image'

export interface SchemaDescriptor {
  kind: SchemaKind
  [field: string]: unknown
}

export interface CollectionConfig<S = unknown> {
  name?: string
  pattern: string | string[]
  baseDir?: string
  single?: boolean
  schema?: SchemaBuilder<S> | SchemaDescriptor
}

export interface OutputOptions {
  data?: string
  assets?: string
  base?: string
  name?: string
  clean?: boolean
  format?: 'esm' | 'cjs'
}

export interface MarkdownOptions {
  gfm?: boolean
  removeComments?: boolean
  copyLinkedFiles?: boolean
  remarkPlugins?: unknown[]
  rehypePlugins?: unknown[]
}

export interface MdxOptions extends MarkdownOptions {
  outputFormat?: 'function-body' | 'module'
  minify?: boolean
}

export interface UserConfig {
  root?: string
  strict?: boolean
  output?: OutputOptions
  collections: Record<string, CollectionConfig>
  loaders?: unknown[]
  markdown?: MarkdownOptions
  mdx?: MdxOptions
  prepare?: (data: Record<string, unknown[]>, ctx: { config: UserConfig }) => unknown
  complete?: (data: Record<string, unknown[]>, ctx: { config: UserConfig }) => unknown
}

export interface BuildCollectionReport {
  name: string
  records: number
  outputPath: string
}

export interface BuildErrorReport {
  file: string
  message: string
}

export interface BuildReport {
  collections: BuildCollectionReport[]
  errors: BuildErrorReport[]
}

interface NativeCollectionInput {
  name: string
  pattern: string
  baseDir: string
  schema?: SchemaDescriptor | null
  single?: boolean
}

interface NativeBuildInput {
  outputDir: string
  collections: NativeCollectionInput[]
  root?: string
  strict?: boolean
  clean?: boolean
  outputAssets?: string | null
  outputBase?: string | null
  outputName?: string | null
  outputFormat?: string | null
  markdownRemarkPlugins?: unknown
  markdownRehypePlugins?: unknown
  mdxRemarkPlugins?: unknown
  mdxRehypePlugins?: unknown
  copyLinkedFiles?: boolean
  mdxOutputFormat?: string
  mdxMinify?: boolean
  markdownGfm?: boolean
}

const cbRegistry = new Map<number, (v: unknown) => unknown>()
let cbId = 0
const registerCallback = (fn: (v: unknown) => unknown): number => {
  const id = ++cbId
  cbRegistry.set(id, fn)
  return id
}

export class SchemaBuilder<_T = unknown> {
  [k: string]: unknown
  constructor(descriptor: SchemaDescriptor) {
    Object.assign(this, descriptor)
  }
  toJSON(): SchemaDescriptor {
    const out: SchemaDescriptor = { kind: this.kind as SchemaKind }
    for (const k of Object.keys(this)) out[k] = this[k]
    return out
  }
  optional(): SchemaBuilder { return new SchemaBuilder({ kind: 'optional', inner: this.toJSON() }) }
  nullable(): SchemaBuilder { return new SchemaBuilder({ kind: 'nullable', inner: this.toJSON() }) }
  default(value: unknown): SchemaBuilder { return new SchemaBuilder({ kind: 'default', inner: this.toJSON(), fallback: value }) }
  min(n: number): SchemaBuilder { return new SchemaBuilder({ ...this.toJSON(), min: n }) }
  max(n: number): SchemaBuilder { return new SchemaBuilder({ ...this.toJSON(), max: n }) }
  length(n: number): SchemaBuilder { return new SchemaBuilder({ ...this.toJSON(), length: n }) }
  regex(p: string): SchemaBuilder { return new SchemaBuilder({ ...this.toJSON(), regex: p }) }
  int(): SchemaBuilder { return new SchemaBuilder({ ...this.toJSON(), int: true }) }
  by(bucket: string): SchemaBuilder { return new SchemaBuilder({ ...this.toJSON(), bucket }) }
  reserved(list: string[]): SchemaBuilder { return new SchemaBuilder({ ...this.toJSON(), reserved: list }) }
  passthrough(): SchemaBuilder { return new SchemaBuilder({ ...this.toJSON(), passthrough: true }) }
  transform(fn: (v: unknown) => unknown): SchemaBuilder {
    return new SchemaBuilder({ kind: 'transform', inner: this.toJSON(), __callbackId: registerCallback(fn) })
  }
  refine(fn: (v: unknown) => boolean, message?: string): SchemaBuilder {
    return new SchemaBuilder({ kind: 'refine', inner: this.toJSON(), __callbackId: registerCallback(fn as (v: unknown) => unknown), __message: message })
  }
}

const sb = (d: SchemaDescriptor): SchemaBuilder => new SchemaBuilder(d)

export interface SBuilders {
  string(): SchemaBuilder<string>
  number(): SchemaBuilder<number>
  boolean(): SchemaBuilder<boolean>
  array<I>(item: SchemaBuilder<I>): SchemaBuilder<I[]>
  object<S extends Record<string, SchemaBuilder>>(fields: S): SchemaBuilder
  record<V>(value: SchemaBuilder<V>): SchemaBuilder<Record<string, V>>
  tuple(items: SchemaBuilder[]): SchemaBuilder<unknown[]>
  intersection<A, B>(a: SchemaBuilder<A>, b: SchemaBuilder<B>): SchemaBuilder<A & B>
  enum<T>(variants: T[]): SchemaBuilder<T>
  literal<T>(value: T): SchemaBuilder<T>
  union<T>(variants: SchemaBuilder<T>[]): SchemaBuilder<T>
  discriminatedUnion<T>(discriminator: string, variants: SchemaBuilder<T>[]): SchemaBuilder<T>
  coerce: {
    string(): SchemaBuilder<string>
    number(): SchemaBuilder<number>
    boolean(): SchemaBuilder<boolean>
    date(): SchemaBuilder<string>
  }
  raw(): SchemaBuilder<string>
  markdown(): SchemaBuilder<string>
  mdx(): SchemaBuilder<string>
  toc(): SchemaBuilder<TocItem[]>
  metadata(): SchemaBuilder<Metadata>
  excerpt(opts?: { length?: number }): SchemaBuilder<string>
  path(opts?: { removeIndex?: boolean }): SchemaBuilder<string>
  slug(bucket?: string, reserved?: string[]): SchemaBuilder<string>
  unique(bucket?: string): SchemaBuilder<string>
  isodate(): SchemaBuilder<string>
  file(opts?: { allowNonRelativePath?: boolean }): SchemaBuilder<string>
  image(opts?: { absoluteRoot?: string }): SchemaBuilder<{ src: string; width: number; height: number }>
}

export const s: SBuilders = {
  string: () => sb({ kind: 'string' }),
  number: () => sb({ kind: 'number' }),
  boolean: () => sb({ kind: 'boolean' }),
  array: (item) => sb({ kind: 'array', item: (item as SchemaBuilder).toJSON() }),
  object: (fields) => sb({
    kind: 'object',
    fields: Object.fromEntries(
      Object.entries(fields).map(([k, v]) => [k, (v as SchemaBuilder).toJSON()]),
    ),
  }),
  record: (value) => sb({ kind: 'record', value: (value as SchemaBuilder).toJSON() }),
  tuple: (items) => sb({ kind: 'tuple', items: items.map((v) => (v as SchemaBuilder).toJSON()) }),
  intersection: (a, b) => sb({ kind: 'intersection', left: (a as SchemaBuilder).toJSON(), right: (b as SchemaBuilder).toJSON() }),
  enum: (variants) => sb({ kind: 'enum', variants }),
  literal: (expected) => sb({ kind: 'literal', expected }),
  union: (variants) => sb({ kind: 'union', variants: variants.map((v) => (v as SchemaBuilder).toJSON()) }),
  discriminatedUnion: (discriminator, variants) => sb({
    kind: 'discriminatedUnion',
    discriminator,
    variants: variants.map((v) => (v as SchemaBuilder).toJSON()),
  }),
  coerce: {
    string: () => sb({ kind: 'coerce.string' }),
    number: () => sb({ kind: 'coerce.number' }),
    boolean: () => sb({ kind: 'coerce.boolean' }),
    date: () => sb({ kind: 'coerce.date' }),
  },
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
  file: (opts = {}) => sb({ kind: 'file', ...opts }),
  image: (opts = {}) => sb({ kind: 'image', ...opts }),
}

export const defineConfig = (config: UserConfig): UserConfig => config
export const defineCollection = <S>(c: CollectionConfig<S>): CollectionConfig<S> => c
export const defineLoader = <L>(l: L): L => l
export const defineSchema = <S>(sch: S): S => sch

interface PendingCallback {
  path: string[]
  kind: 'transform' | 'refine'
  fn: (v: unknown) => unknown
  message?: string
}

function collectCallbacks(descriptor: SchemaDescriptor | undefined, base: string[] = []): PendingCallback[] {
  if (!descriptor || typeof descriptor !== 'object') return []
  const found: PendingCallback[] = []
  if (descriptor.kind === 'transform' && typeof descriptor.__callbackId === 'number') {
    const fn = cbRegistry.get(descriptor.__callbackId as number)
    if (fn) found.push({ path: [...base], kind: 'transform', fn })
  }
  if (descriptor.kind === 'refine' && typeof descriptor.__callbackId === 'number') {
    const fn = cbRegistry.get(descriptor.__callbackId as number)
    if (fn) found.push({ path: [...base], kind: 'refine', fn, message: descriptor.__message as string | undefined })
  }
  if (descriptor.inner) found.push(...collectCallbacks(descriptor.inner as SchemaDescriptor, base))
  if (descriptor.kind === 'object' && descriptor.fields) {
    for (const [k, v] of Object.entries(descriptor.fields as Record<string, SchemaDescriptor>)) {
      found.push(...collectCallbacks(v, [...base, k]))
    }
  }
  if (descriptor.kind === 'array' && descriptor.item) {
    found.push(...collectCallbacks(descriptor.item as SchemaDescriptor, [...base, '*']))
  }
  return found
}

interface PathTarget { parent: Record<string, unknown>; key: string }

function walkPath(obj: unknown, path: string[]): PathTarget[] {
  if (path.length === 0) return []
  if (path[0] === '*') {
    if (!Array.isArray(obj)) return []
    return obj.flatMap((_, i) => walkPath((obj as unknown[])[i], path.slice(1)))
  }
  const [key, ...rest] = path
  if (obj == null || typeof obj !== 'object' || !(key in (obj as Record<string, unknown>))) return []
  if (rest.length === 0) return [{ parent: obj as Record<string, unknown>, key }]
  return walkPath((obj as Record<string, unknown>)[key], rest)
}

function applyCallbacks(record: unknown, cbs: PendingCallback[], errors: BuildErrorReport[], file: string): void {
  for (const cb of cbs) {
    for (const { parent, key } of walkPath(record, cb.path)) {
      const v = parent[key]
      if (cb.kind === 'transform') {
        try { parent[key] = cb.fn(v) }
        catch (e) {
          errors.push({ file, message: `${cb.path.join('.')}: transform threw: ${(e as Error).message ?? e}` })
        }
      } else {
        let ok = false
        try { ok = !!cb.fn(v) }
        catch (e) {
          errors.push({ file, message: `${cb.path.join('.')}: refine threw: ${(e as Error).message ?? e}` })
          continue
        }
        if (!ok) errors.push({ file, message: `${cb.path.join('.')}: ${cb.message ?? 'failed refinement'}` })
      }
    }
  }
}

function adaptToBuildInput(input: UserConfig | NativeBuildInput): NativeBuildInput {
  if ('outputDir' in input && Array.isArray(input.collections)) return input as NativeBuildInput
  const cfg = input as UserConfig
  const root = cfg.root ?? '.'
  const outputDir = cfg.output?.data ?? '.gentleduck'
  const collections: NativeCollectionInput[] = Object.entries(cfg.collections ?? {}).map(([key, c]) => ({
    name: c.name ?? key,
    pattern: Array.isArray(c.pattern) ? c.pattern[0] : c.pattern,
    baseDir: c.baseDir ?? root,
    schema: c.schema instanceof SchemaBuilder ? c.schema.toJSON() : (c.schema as SchemaDescriptor | undefined),
    single: c.single,
  }))
  return {
    outputDir,
    collections,
    root,
    strict: cfg.strict,
    clean: cfg.output?.clean,
    outputAssets: cfg.output?.assets,
    outputBase: cfg.output?.base,
    outputName: cfg.output?.name,
    outputFormat: cfg.output?.format,
    markdownRemarkPlugins: cfg.markdown?.remarkPlugins,
    markdownRehypePlugins: cfg.markdown?.rehypePlugins,
    mdxRemarkPlugins: cfg.mdx?.remarkPlugins,
    mdxRehypePlugins: cfg.mdx?.rehypePlugins,
    copyLinkedFiles: cfg.markdown?.copyLinkedFiles ?? cfg.mdx?.copyLinkedFiles,
    mdxOutputFormat: cfg.mdx?.outputFormat,
    mdxMinify: cfg.mdx?.minify,
    markdownGfm: cfg.markdown?.gfm,
  }
}

export function compile(source: string): CompileOutput {
  return native.compile(source) as CompileOutput
}

export function compileMany(sources: string[]): CompileOutput[] {
  return native.compileMany(sources) as CompileOutput[]
}

export async function build(input: UserConfig): Promise<BuildReport> {
  const collectionCallbacks = new Map<string, PendingCallback[]>()
  if (input?.collections && !Array.isArray(input.collections)) {
    for (const [key, c] of Object.entries(input.collections)) {
      if (c.schema) {
        const desc = c.schema instanceof SchemaBuilder ? c.schema.toJSON() : (c.schema as SchemaDescriptor)
        const cbs = collectCallbacks(desc)
        if (cbs.length) collectionCallbacks.set(c.name ?? key, cbs)
      }
    }
  }

  const report = native.build(adaptToBuildInput(input)) as BuildReport

  const needPostprocess = collectionCallbacks.size > 0 || input.prepare || input.complete
  if (!needPostprocess) return report

  const data: Record<string, unknown[]> = {}
  for (const c of report.collections) {
    data[c.name] = JSON.parse(readFileSync(c.outputPath, 'utf8'))
  }

  for (const c of report.collections) {
    const cbs = collectionCallbacks.get(c.name)
    if (!cbs) continue
    const records = Array.isArray(data[c.name]) ? data[c.name] : [data[c.name]]
    for (const record of records) {
      applyCallbacks(record, cbs, report.errors, c.outputPath)
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
  return report
}

export default {
  compile,
  compileMany,
  build,
  defineConfig,
  defineCollection,
  defineLoader,
  defineSchema,
  s,
  SchemaBuilder,
}
