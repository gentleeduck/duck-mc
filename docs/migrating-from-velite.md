# Migrating from velite

`@duck/md` mirrors velite's public API. For the common case, change
the import and you're done.

```diff
- import { defineConfig, s } from 'velite'
+ import { defineConfig, s } from '@duck/md'
```

## What works the same

| velite                          | @duck/md           |
| ------------------------------- | ------------------ |
| `defineConfig({...})`           | identical          |
| `output.data` / `output.clean`  | identical          |
| `output.assets` / `output.base` / `output.name` template | identical |
| `collections.<key>.{name,pattern,schema,single}` | identical |
| `s.string()`, `.max(N)`, `.min(N)`, `.regex(p)` | identical |
| `s.number()`, `s.boolean()`, `s.array()`, `s.object()` | identical |
| `s.enum([...])`, `s.literal(v)`, `s.union([...])` | identical |
| `s.optional()`, `.nullable()`, `.default(v)` | identical |
| `s.markdown()`, `s.mdx()`, `s.raw()`, `s.toc()`, `s.metadata()`, `s.excerpt()` | identical |
| `s.path()`, `s.slug()`, `s.unique()`, `s.isodate()` | identical |
| `s.file()`, `s.image()`         | identical (returns `{src,width,height}`; blur deferred) |
| `--strict`, `--clean`, `--watch` (`dev`) | identical |
| `.velite/` output (or any path you point at) | identical |
| `index.js` + typed `index.d.ts` | identical          |

## Differences to know

| Area                        | velite                                    | @duck/md                            |
| --------------------------- | ----------------------------------------- | ----------------------------------- |
| Image blur dataURL          | base64 webp via sharp                     | dimensions only (deferred)          |
| Word-level marks `/word/`   | yes                                       | line marks only `{1,3-5}`           |
| Dual themes                 | shiki paired output                       | single syntect theme                |
| `.transform()` / `.refine()` (JS callback) | runs during JS build      | accepted in Rust API; JS callback bridge deferred |
| `prepare()` / `complete()` hooks | runs                                | Rust accepts but no-op (TODO)       |
| Custom JS plugins (remark/rehype) | run inline                          | spawns `@duck/md-sidecar` subprocess |
| Watch mode                  | chokidar                                  | notify (Linux/macOS/Windows)        |

## Plugin compatibility

By default duck-md runs a native Rust pipeline (no Node child process
needed): `code_import`, `npm_command`, `bare_url`, `autolink_headings`
(with `subheading-anchor` class), `pretty_code` (single theme).

If you need community plugins (`rehype-pretty-code`, `rehype-slug`,
`remark-toc`, etc.), pass them through `markdown.remarkPlugins` /
`mdx.rehypePlugins`. duck-md detects them and shells out to
`@duck/md-sidecar` which runs them via unified.

## CLI

```sh
# build once
duck-md build --config duck-md.config.ts

# watch + rebuild
duck-md dev --config duck-md.config.ts

# strict (fail on first validation error)
duck-md build --strict
```

Velite users: `velite build` → `duck-md build`. `velite dev` → `duck-md dev`.

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
