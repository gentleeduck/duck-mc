export interface TocItem {
  title: string
  url: string
  items: TocItem[]
}

export interface Metadata {
  reading_time: number
  word_count: number
}

export interface CompileOutput {
  body: string
  content: string
  html: string
  excerpt: string
  metadata: Metadata
  toc: TocItem[]
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

export declare function compile(source: string): CompileOutput
export declare function build(input: BuildInput): BuildReport
export declare function defineConfig(config: BuildInput): BuildInput
