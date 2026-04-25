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
  | 'array' | 'object'
  | 'enum' | 'literal' | 'union'
  | 'optional' | 'nullable' | 'default' | 'transform' | 'refine'
  | 'raw' | 'markdown' | 'mdx' | 'toc' | 'metadata' | 'excerpt'
  | 'path' | 'slug' | 'unique' | 'isodate'

export interface SchemaDescriptor {
  kind: SchemaKind
  [field: string]: unknown
}

export interface Schema<_T = unknown> extends SchemaDescriptor {
  optional(): Schema<_T | undefined>
  nullable(): Schema<_T | null>
  default(value: _T): Schema<_T>
  min(n: number): Schema<_T>
  max(n: number): Schema<_T>
  length(n: number): Schema<_T>
  regex(pattern: string): Schema<_T>
  int(): Schema<_T>
  by(bucket: string): Schema<_T>
  reserved(list: string[]): Schema<_T>
  passthrough(): Schema<_T>
}

export interface SchemaBuilder {
  string(): Schema<string>
  number(): Schema<number>
  boolean(): Schema<boolean>
  array<I>(item: Schema<I>): Schema<I[]>
  object<S extends Record<string, Schema<unknown>>>(fields: S): Schema<{ [K in keyof S]: unknown }>
  enum<T>(variants: T[]): Schema<T>
  literal<T>(value: T): Schema<T>
  union<T>(variants: Schema<T>[]): Schema<T>

  raw(): Schema<string>
  markdown(): Schema<string>
  mdx(): Schema<string>
  toc(): Schema<TocItem[]>
  metadata(): Schema<{ readingTime: number; wordCount: number }>
  excerpt(opts?: { length?: number }): Schema<string>
  path(opts?: { removeIndex?: boolean }): Schema<string>
  slug(bucket?: string, reserved?: string[]): Schema<string>
  unique(bucket?: string): Schema<string>
  isodate(): Schema<string>
}

export const s: SchemaBuilder

export interface CollectionConfig {
  name: string
  pattern: string | string[]
  baseDir?: string
  single?: boolean
  schema?: Schema<unknown>
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

export interface CollectionInput {
  name: string
  pattern: string
  baseDir: string
}

export interface BuildInput {
  outputDir: string
  collections: CollectionInput[]
}

export interface BuildCollectionReport {
  name: string
  records: number
  outputPath: string
}

export interface BuildReport {
  collections: BuildCollectionReport[]
}

export declare function compile(source: string): CompileOutput
export declare function compileMany(sources: string[]): CompileOutput[]
export declare function build(input: BuildInput | UserConfig): Promise<BuildReport>

export declare function defineConfig(config: UserConfig): UserConfig
export declare function defineCollection(c: CollectionConfig): CollectionConfig
export declare function defineLoader<L>(l: L): L
export declare function defineSchema<S>(s: S): S
