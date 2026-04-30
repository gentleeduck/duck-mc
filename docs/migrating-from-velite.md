# Migrating from velite

`@duck/md` mirrors velite's public API. For the common case, change
the import and you're done.

```diff
- import { defineConfig, s } from 'velite'
+ import { defineConfig, s } from '@duck/md'
```

## What works the same

| velite                                                                         | @duck/md                                                |
| ------------------------------------------------------------------------------ | ------------------------------------------------------- |
| `defineConfig({...})`                                                          | identical                                               |
| `output.data` / `output.clean`                                                 | identical                                               |
| `output.assets` / `output.base` / `output.name` template                       | identical                                               |
| `collections.<key>.{name,pattern,schema,single}`                               | identical                                               |
| `s.string()`, `.max(N)`, `.min(N)`, `.regex(p)`                                | identical                                               |
| `s.number()`, `s.boolean()`, `s.array()`, `s.object()`                         | identical                                               |
| `s.enum([...])`, `s.literal(v)`, `s.union([...])`                              | identical                                               |
| `s.optional()`, `.nullable()`, `.default(v)`                                   | identical                                               |
| `s.markdown()`, `s.mdx()`, `s.raw()`, `s.toc()`, `s.metadata()`, `s.excerpt()` | identical                                               |
| `s.path()`, `s.slug()`, `s.unique()`, `s.isodate()`                            | identical                                               |
| `s.file()`, `s.image()`                                                        | identical (returns `{src,width,height}`; blur deferred) |
| `--strict`, `--clean`, `--watch` (`dev`)                                       | identical                                               |
| `.velite/` output (or any path you point at)                                   | identical                                               |
| `index.js` + typed `index.d.ts`                                                | identical                                               |

## Differences to know

| Area                                                   | velite                        | @duck/md                                  |
| ------------------------------------------------------ | ----------------------------- | ----------------------------------------- |
| Image blur dataURL                                     | base64 webp via sharp         | base64 webp via `image` crate ✅          |
| Word-level marks `/word/`                              | yes                           | yes ✅                                    |
| Line marks `{1,3-5}`                                   | yes                           | yes ✅                                    |
| Dual themes                                            | shiki paired output           | syntect paired output ✅                  |
| Mermaid                                                | headless Chromium             | shells out to `mmdc` if on PATH ✅        |
| `<ComponentSource path=>`                              | reads + emits tsx code block  | same ✅                                   |
| `<ComponentPreview name=>`                             | registry lookup + tsx         | yes ✅ (accepts JSON registry index)      |
| Indented code blocks                                   | yes                           | yes ✅                                    |
| `<email>` autolinks                                    | yes                           | yes ✅ (auto-prefixes mailto:)            |
| `.transform()` / `.refine()` (JS callback)             | runs during JS build          | yes ✅ (FFI bridge via callback registry) |
| `s.regex()` enforcement                                | yes                           | yes ✅ (regex crate)                      |
| `s.record / tuple / intersection / discriminatedUnion` | yes (zod fork)                | yes ✅                                    |
| `s.coerce.{string,number,boolean,date}`                | yes                           | yes ✅                                    |
| `.superRefine()`                                       | yes                           | yes ✅                                    |
| `prepare()` / `complete()` hooks                       | runs (async)                  | runs (async via JS adapter) ✅            |
| Custom JS plugins (remark/rehype)                      | run inline                    | spawns `dmc-sidecar` ✅               |
| Watch mode                                             | chokidar                      | notify ✅                                 |
| Per-file compile parallelism                           | sequential                    | rayon parallel ✅                         |
| `build()` return                                       | `Promise<Report>`             | `Promise<Report>` ✅                      |
| `markdown.copyLinkedFiles`                             | yes                           | yes ✅ (asset hash + rewrite)             |
| `mdx.outputFormat: 'module'`                           | yes                           | yes ✅ (ESM module wrap)                  |
| `mdx.minify`                                           | yes (terser)                  | yes ✅ (whitespace collapse)              |
| `markdown.gfm: false` toggle                           | yes                           | yes ✅ (DisableGfm post-pass)             |
| Grapheme-aware columns                                 | n/a                           | yes ✅ (unicode-segmentation)             |
| Parser error recovery                                  | yes (vfile-reporter messages) | yes ✅ (Document.diagnostics)             |
| Multi-platform binaries                                | n/a (pure JS)                 | 7 targets via napi-rs ✅                  |
| Fuzz targets                                           | n/a                           | yes ✅ (cargo-fuzz)                       |

## Remaining limitations

- **Custom user-defined `loaders[]` registration** — built-in matter / yaml / json loaders work; user-defined `{ test, load }` loaders accepted in config but not invoked. Workaround: pre-process source files in a build script before running `dmc build`, or add a `prepare(data)` hook to mutate records post-validation.
- **Byte-exact velite output equivalence** — output shape matches velite (camelCase fields, hoisted frontmatter, typed `.d.ts`). Body strings differ when minify is on (whitespace collapse vs terser AST minify) but produce equivalent JS.
- **Schema `.regex(p)` uses Rust regex syntax** — minor differences vs JavaScript regex (e.g. no lookbehind by default).

## Plugin compatibility

By default dmc runs a native Rust pipeline (no Node child process
needed): `code_import`, `npm_command`, `bare_url`, `autolink_headings`
(with `subheading-anchor` class), `pretty_code` (single + dual theme),
`mermaid`, `ComponentSource`, `ComponentPreview`, `copy_linked_files`.

If you need community plugins (`rehype-pretty-code`, `rehype-slug`,
`remark-toc`, etc.), pass them through `markdown.remarkPlugins` /
`mdx.rehypePlugins`. dmc detects them and shells out to
`@duck/md-sidecar` which runs them via unified.

## CLI

```sh
# build once
dmc build --config dmc.config.ts

# watch + rebuild
dmc dev --config dmc.config.ts

# strict (fail on first validation error)
dmc build --strict
```

Velite users: `velite build` → `dmc build`. `velite dev` → `dmc dev`.

## Output schema

Per record (matches `velite` byte-for-byte except where noted in the
table above):

```ts
{
  title: string,
  description?: string,
  // ...other frontmatter fields hoisted to root...
  body: string,                // jsx/jsxs factory function source
  content: string,             // raw markdown body
  excerpt: string,             // ~260 char plaintext
  metadata: { readingTime: number, wordCount: number },
  toc: TocItem[],
  contentType: string,
  flattenedPath: string,
  permalink: string,
  slug: string,
  sourceFileDir: string,
  sourceFileName: string,
  sourceFilePath: string,
}
```

The `body` is a JS function-source string. Run it via
`new Function(body)(jsxRuntime, components)` (see
`examples/web/src/MdxContent.tsx` for the full runtime).
