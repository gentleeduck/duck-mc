# Migrating from velite

dmc's API is intentionally close to velite's. Most configs port with
minimal edits.

## Imports

```diff
- import { defineConfig, s } from "velite";
+ import { defineConfig, s } from "@gentleduck/md";
```

## Output dir

```diff
  output: {
-   data: ".velite",
+   data: ".gentleduck",
  }
```

Output filename pattern (`[name]-[hash:6].[ext]`) and asset base
(`/assets/`) are the same default.

## Schema builder

`s` is a near-drop-in for velite's `s`. Same Zod shape, same `.path()`
helper, same `.markdown()` and `.mdx()` schemas.

```ts
schema: s.object({
  title: s.string().max(99),
  date: s.date(),
  slug: s.path(),
  html: s.markdown(),
});
```

`s.markdown()` runs the dmc compile pipeline per record (transformer
chain native; sidecar gated as documented).

## Plugins

```ts
markdown: {
  remarkPlugins: [remarkGfm, remarkMath],
  rehypePlugins: [rehypeKatex, rehypePrettyCode, rehypeSlug, rehypeAutolinkHeadings],
}
```

Works as-is. dmc's plugin gate strips:

- `remark-gfm` (handled natively by parser)
- `remark-math` (handled by Math transformer)
- `remark-emoji` (handled by Emoji)
- `rehype-pretty-code`, `shiki` (handled by PrettyCode)
- `rehype-katex`, `rehype-mathjax` (handled by Math)
- `rehype-slug`, `rehype-autolink-headings` (handled by AutolinkHeadings)

Plugins not in that list run via the dmc-sidecar Node child.

### Want to keep one of those JS plugins anyway?

Use `preferSidecar` to keep a specific plugin in the JS chain and
drop the matching native transformer:

```ts
markdown: {
  rehypePlugins: [
    [rehypeKatex, { strict: false }],
  ],
  preferSidecar: ["rehype-katex"],   // velite-style katex, no native Math
}
```

Or `forceSidecar: true` to mirror velite's plugin chain entirely
(every JS plugin runs in sidecar, every native is dropped). See
[`plugins.md`](./plugins.md#override-the-gate-force-js-plugins).

## Hooks

```ts
prepare(data) { ... }
complete(data) { ... }
```

Same signatures, same firing order (after schema validation, after
all collections written).

## Build script

```diff
- "content": "velite build",
+ "content": "tsx scripts/build-content.ts",
```

```ts
// scripts/build-content.ts
import { build } from "@gentleduck/md";
import config from "../duck-md.config";

await build(config);
```

Or use the CLI:

```diff
- "content": "velite build",
+ "content": "dmc build",
```

## Output shape

Same JSON shape. `<output_dir>/<name>.json` is an array of validated
records. `<output_dir>/index.js` re-exports them.

## Differences worth knowing

| feature | velite | dmc |
|---------|--------|-----|
| persistent cache | none (memory only) | yes, `<output>/.cache/` |
| native math | no (rehype-katex JS) | yes (KaTeX or MathML) |
| native syntax highlight | no (rehype-pretty-code JS) | yes (syntect) |
| sidecar pool | n/a (in-process JS) | yes (Node child pool) |
| watch mode | yes | yes (`dmc dev`) |
| MDX output | yes (`.mdx` -> JS body) | yes (`MdxBodyEmitter`) |
| schema builder | Zod | Zod-style (same surface) |

## Bench

Same content (`examples/nextjs/content/docs/kitchen-sink.mdx`):

| | dmc | velite |
|-------|-----|--------|
| cold | 145 ms | 1380 ms |
| warm | 334 ms | 1380 ms (no cache) |

dmc kitchen-sink @N=1000: **9.5x faster** with caches off, **4.1x
faster** with cache warm vs velite cold.

## Removing velite

```diff
- "velite": "^0.3"
+ "@gentleduck/md": "^0.1"
```

```bash
rm -rf .velite/
pnpm add @gentleduck/md
pnpm dmc build
```

Imports update from `.velite` to `.gentleduck`. Type signatures stay
identical (Zod schema -> typed records).
