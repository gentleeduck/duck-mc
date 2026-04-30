export interface TocItem { title: string; url: string; items: TocItem[] }
export interface Metadata { readingTime: number; wordCount: number }
export interface DocRecord {
  body: string
  content: string
  excerpt: string
  metadata: Metadata
  toc: TocItem[]
  contentType: string
  flattenedPath: string
  permalink: string
  slug: string
  sourceFileDir: string
  sourceFileName: string
  sourceFilePath: string
  [frontmatterField: string]: unknown
}
export declare const docs: DocRecord[]
