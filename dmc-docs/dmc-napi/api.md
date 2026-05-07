# dmc-napi API

TypeScript signatures from `mod.ts` and `index.d.ts`.

## `defineConfig`

```ts
export function defineConfig(cfg: UserConfig): UserConfig
```

Identity. Pure type narrowing. Wrap your config so editor inference
flows through.

## `s` (schema builder)

```ts
export const s: {
  object<T extends Record<string, ZodTypeAny>>(shape: T): ZodObject<T>;
  string(): ZodString;
  number(): ZodNumber;
  boolean(): ZodBoolean;
  array<T extends ZodTypeAny>(item: T): ZodArray<T>;
  union<T extends ZodTypeAny[]>(opts: T): ZodUnion<T>;
  literal<T extends string | number | boolean>(v: T): ZodLiteral<T>;
  enum<T extends [string, ...string[]]>(values: T): ZodEnum<T>;
  date(): ZodDate;
  path(): ZodString;
  // ... full Zod surface
};
```

Used inside collection schemas:

```ts
schema: s.object({
  title: s.string().max(99),
  date: s.date(),
  tags: s.array(s.string()).optional(),
});
```

## `build(cfg)`

```ts
export function build(cfg: UserConfig): Promise<BuildReport>
```

Runs the engine end to end. Reads matched files, compiles each in
parallel via Rayon, runs sidecar if foreign plugins are listed,
validates frontmatter, writes `<output>/<name>.json` + index.

```ts
type BuildReport = {
  collections: Array<{
    name: string;
    records: number;
    outputPath: string;
  }>;
  errors: Array<{ file: string; message: string }>;
};
```

## `compile(source)`

```ts
export function compile(source: string): CompileOutput
```

One-shot synchronous compile of an MDX string. Uses defaults
(`CompileConfig::new`). Skips file-cache + sidecar.

```ts
type CompileOutput = {
  frontmatter: unknown;
  frontmatterRaw: string;
  content: string;
  html: string;
  body: string;
  excerpt: string;
  metadata: { readingTime: number; wordCount: number };
  toc: TocItem[];
  imports: string[];
  exports: string[];
};
```

## `compileMany(sources)`

```ts
export function compileMany(sources: string[]): CompileOutput[]
```

Same as `compile` but reuses the lexer/parser engine across the batch.
Slightly faster for many small inputs.

## `latexToHtml(latex, display)`

```ts
export function latexToHtml(latex: string, display: boolean): string
```

Direct KaTeX render via the embedded engine. Useful for JS-side
preprocessing without running a full compile.

## Config interfaces

```ts
interface UserConfig {
  root: string;
  output?: OutputOptions;
  clean?: boolean;
  strict?: boolean;
  cacheEnabled?: boolean;
  collections: Record<string, CollectionInput>;
  markdown?: MarkdownOptions;
  mdx?: MdxOptions;
  prepare?: (data: Record<string, unknown[]>) => unknown;
  complete?: (data: Record<string, unknown[]>) => unknown;
  loaders?: Loader[];
}

interface OutputOptions {
  data?: string;       // dir for <name>.json (default ".gentleduck")
  assets?: string;     // copy_linked_files target dir
  base?: string;       // public URL prefix
  name?: string;       // file naming pattern
  clean?: boolean;     // wipe before build
  format?: "esm" | "cjs";
  html?: boolean;      // include rendered HTML in record
}

interface MarkdownOptions {
  gfm?: boolean;
  removeComments?: boolean;
  copyLinkedFiles?: boolean;
  remarkPlugins?: Pluggable[];   // unified plugin tuples
  rehypePlugins?: Pluggable[];

  // Plugin gate overrides:
  forceSidecar?: boolean;        // every JS plugin in sidecar, all natives dropped
  preferSidecar?: string[];      // per-plugin sidecar override (see list below)
}

interface MdxOptions extends MarkdownOptions {
  outputFormat?: "function-body" | "module";
  minify?: boolean;
}
```

`preferSidecar` recognises the same names the gate strips:

| name | native dropped |
| --- | --- |
| `remark-gfm` | parser GFM (sets `markdownGfm = false`) |
| `remark-math`, `rehype-katex`, `rehype-mathjax` | `Math` |
| `remark-emoji` | `Emoji` |
| `rehype-pretty-code`, `shiki` | `PrettyCode` |
| `rehype-slug`, `rehype-autolink-headings` | `AutolinkHeadings` |

```ts
interface CollectionInput {
  name: string;
  pattern: string;
  schema?: ZodTypeAny;
  single?: boolean;
}
```

## Re-exported types

```ts
export type { Plugin, Pluggable } from "unified";
export type { Loader, Result } from "./loaders";
export type { CompileOutput, BuildReport };
```
