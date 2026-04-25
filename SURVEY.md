# Velite Parity Survey (target: duck-ui docs)

Source of truth for what `duck-md` must reproduce. Read this before designing schema, transformers, or output shape. Pulled from:

- `packages/duck-docs/src/velite/config.ts`
- `packages/duck-docs/src/velite/plugins/*`
- `apps/duck/velite.config.ts`
- `apps/duck/velite-configs/{index.ts,plugins/rehype-component.ts}`
- `apps/duck/.velite/{index.js,index.d.ts,docs.json}`

## A. Output schema per docs item

| Field | Type | Derivation |
|---|---|---|
| `body` | string (compiled MDX function source — uses `arguments[0]`, `Fragment`, `jsx`, `jsxs`) | `s.mdx()` |
| `component` | bool | `s.boolean().default(false)` from frontmatter |
| `content` | string (raw markdown) | `s.markdown()` |
| `description` | string | `s.string()` from frontmatter |
| `excerpt` | string (~260 char) | `s.excerpt()` |
| `links` | `{ api?, doc? }` optional | `s.object({...}).optional()` |
| `metadata` | `{ readingTime, wordCount }` | `s.metadata()` |
| `title` | string ≤99 | `s.string().max(99)` |
| `toc` | nested `{title,url,items[]}` | `s.toc()` then `cleanTocItems` |
| `contentType` | string | `path.split('.').pop()` |
| `flattenedPath` | string | second-to-last seg minus `.mdx` |
| `permalink` | string | `path.replace(/^.*docs\//,'').replace(/\.mdx$/,'')` |
| `slug` | string | `'docs/' + permalink` or `'docs'` |
| `sourceFileDir` | string | last 2 dir segs joined |
| `sourceFileName` | string | basename |
| `sourceFilePath` | string | `path` arg from velite (observed empty in current build — quirk to mirror) |

## B. Velite primitives in use

- `s.mdx()` → JS function source string (CommonJS-ish)
- `s.markdown()` → raw md text
- `s.excerpt()` → short plaintext excerpt
- `s.metadata()` → `{readingTime, wordCount}`
- `s.toc()` → nested heading tree
- `s.string() / s.string().max(N)` / `s.boolean().default(false)` / `s.object({}).optional()` — frontmatter
- Top-level wrapper: `s.object({...}).transform(fn)` — `fn` injects path-derived fields

## C. Pipeline order

Remark (mdast):
1. `...remarkPluginsBefore`
2. `remark-gfm`
3. `remark-code-import` (resolves `file=...` meta, inlines)
4. `...remarkPlugins`

Rehype (hast), in `apps/duck/velite.config.ts` order:
1. `rehypeComponent` (local, before)
2. `rehype-slug`
3. `rehypeMetadataPlugin` (local)
4. `rehype-pretty-code` (Shiki, dual themes catppuccin-mocha + github-light, line/word callbacks)
5. `rehypeTitle` (local)
6. `rehypePreBlockSource` (local)
7. `rehypeMermaid` (local)
8. `rehypeNpmCommand` (local)
9. `rehype-autolink-headings` (className `subheading-anchor`, ariaLabel)

## D. Custom plugin behaviors

- `rehypeMetadataPlugin` — visits `<code>`, parses fence meta `title="..."` and `/word/`, sets `__rawString__`, `__title__`, `__marks__`
- `rehypeTitle` — renames `<div data-rehype-pretty-code-title>` → `<figcaption>`
- `rehypePreBlockSource` — copies `__rawString__` onto each `<pre>` in the pretty-code fragment
- `rehypeMermaid` — finds `<MermaidDiagram chart=...>`/mermaid `<pre>`/wrapped pre, batch-renders with headless Chromium → injects `lightSvg`/`darkSvg` (or `__mermaidLightSvg__`/`__mermaidDarkSvg__`)
- `rehypeNpmCommand` — for `<pre>` whose `__rawString__` starts with `npm install` / `npx create-` / `npx`: synthesize yarn/pnpm/bun equivalents
- `rehypeComponent` (app-local) — handles `<ComponentSource path=>` and `<ComponentPreview name=>`. Reads `public/r/index.json`, follows `packagesRoot/<source>` plus `registry-examples/src` / `registry-internals/src` fallbacks; rewrites `@gentleduck/registry-ui` → `~/components`, `export default` → `export`; emits a `tsx` code block

## E. Collections + globs

Single `docs` collection. Default pattern `docs/**/*.mdx`. App match resolves both `content/docs/**` and `docs/**`.

## F. Output artifacts

`.velite/`:
- `docs.json` — array of records (schema A)
- `index.js` — `export { default as docs } from './docs.json' with { type: 'json' }`
- `index.d.ts` — `type Docs = Collections['docs']['schema']['_output']; declare const docs: Docs[]`

## G. Replacement notes (npm → Rust)

- `velite` (framework) → custom Rust pipeline
- `@mdx-js/mdx` (powers `s.mdx()`) → no pure-Rust MDX. Options: (a) emit raw + JSX-as-data, consumer compiles; (b) hand-build a JS function-source emitter from our AST (no JS eval); (c) sidecar Node call. Plan: (b).
- `unified` / `unist-util-visit` / `unist-builder` → custom mdast/hast visitors over our AST
- `remark-gfm` → port behaviors (tables, task lists, strikethrough, autolinks) into our parser
- `remark-code-import` → small Rust fn reading `file=` meta + inlining
- `rehype-slug` → `slug` crate over heading text
- `rehype-pretty-code` + Shiki → `syntect` w/ theme JSON ports (catppuccin-mocha + github-light); replicate dual-theme paired `<pre>` shape under `data-rehype-pretty-code-fragment`
- `rehype-autolink-headings` → manual `<a class=subheading-anchor>` injection after slug
- `mermaid` (headless Chromium) → optional sidecar `mmdc`, otherwise pre-render upstream and cache results
- `s.metadata()` (reading time) → `reading_time` crate or `words/200` formula
- `s.excerpt()` → first N chars of plaintext-stripped markdown
- `s.toc()` → walk headings, build nested by depth
- frontmatter → `serde_yaml` + validator

## H. Hardest behaviors

1. `s.mdx()` body output — must emit valid JS factory. No JS engine in our pipeline. Hand-build emitter for the `Fragment, jsx, jsxs` calling convention.
2. MDX JSX expression eval (mermaid plugin uses `new Function`) — restrict our impl to literal/template-literal cases.
3. estree payloads inside hast — we do not support full estree round-trip; serialize JSX expression text only.
4. Shiki dual-theme highlight + line/word marks — replicate using syntect with synthesized class names (`line--highlighted`, `word--highlighted`) and `{1,3-5}` mark parsing.
5. Headless-Chromium mermaid — out of scope for pure-Rust. Plan: optional `mmdc` shell-out behind feature flag.

## I. Output contract for our `body` field

To match consumer expectations without a JS engine, our `body` must be a string of the shape:

```js
function _createMdxContent(props) {
  const _components = { ...props.components };
  const { Fragment } = arguments[0];
  const { jsx, jsxs } = arguments[0];
  return jsxs(Fragment, { children: [
    jsx("h1", { id: "...", children: "..." }),
    ...
  ]});
}
return _createMdxContent(arguments[0]);
```

Wrap markdown elements + JSX components into `jsx`/`jsxs` calls. Components referenced by name (capitalized identifier) are passed through as identifiers — caller resolves them via `props.components` or scope.
