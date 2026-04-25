// hand-written TypeScript surface for @duck/md.
// Re-exports the napi binding (./index.js) with rich types and a defineConfig helper.

export interface TocItem {
  title: string
  url: string
  items: TocItem[]
}

export interface Metadata {
  /** Reading time in minutes (200 wpm). */
  reading_time: number
  word_count: number
}

export interface CompileOutput {
  /** MDX body — JS factory function source compatible with `react/jsx-runtime`. */
  body: string
  /** Raw markdown body (frontmatter stripped). */
  content: string
  /** Rendered HTML for static rendering / SSR fallbacks. */
  html: string
  /** Short plaintext excerpt (~260 chars). */
  excerpt: string
  metadata: Metadata
  toc: TocItem[]
  /** Parsed frontmatter object (or null when absent). */
  frontmatter: unknown
  frontmatter_raw: string
  imports: string[]
  exports: string[]
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

/** Compile a single MDX source string. */
export declare function compile(source: string): CompileOutput

/**
 * Walk globs, compile each MDX file, write velite-shape JSON to `outputDir`.
 * Equivalent to the `duck-md build` CLI, but driven by a JS object — no toml needed.
 */
export declare function build(input: BuildInput): BuildReport

/** Identity helper for IDE autocompletion on config objects. */
export declare function defineConfig(config: BuildInput): BuildInput
