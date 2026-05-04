# FAQ

Top hits, terse answers.

## Why is my build slow on first run?

Cold builds spend ~25-100 ms loading the syntect SyntaxBundle (themes
+ grammars) plus ~50-200 ms warming the KaTeX quick-js engine when
math is on. Persistent cache makes second runs 3.55x faster.

## Why are my warm rebuilds not faster?

Check `<output_dir>/.cache/dmc/` exists. If missing:

- `cacheEnabled: false` in config -> set true.
- Cache wiped by `clean: true` -> remove or move to a different dir.
- CI agent starts fresh -> persist `.cache/` in the build cache.

## Math renders but looks plain. Why?

Two cases:

- `mathEngine: "mathml"` is on. Browser MathML rendering varies; layout
  is plainer than KaTeX. Switch to `"katex"` (default) and ship
  `katex.min.css` on the page.
- KaTeX is on but the CSS is not loaded. Add the CDN link to your
  layout (see `nextjs.md`).

## Code blocks have no color. Why?

Three cases:

- `pretty-code` feature is off at compile time (slim binary). Enable
  via Cargo features.
- Multi-theme output is on but consumer CSS does not map
  `--dmc-{mode}` to `color` for the active mode. Add the rule (see
  `dmc-transform/transformers/pretty-code.md`).
- The grammar bundle does not include the language. Falls back to
  plain text; tokens render as raw text. Add the grammar (see
  `dmc-highlight/grammars.md`).

## Can I add my own transformer?

Yes. See `dmc-transform/writing-a-transformer.md`. Implement
`Transformer`, walk via `walk_root`, mutate via `Visitor`.

## Can I run dmc and velite in the same project?

Yes. They write to different output dirs (`.gentleduck` vs
`.velite`). The two example apps in `examples/` do exactly that for
side-by-side comparison.

## Does dmc replace velite?

Functionally, yes (Zod-style schema, frontmatter validation, plugin
chain via sidecar, watch mode, output JSON shape). Plus: persistent
cache, native transformers (math / highlight / emoji / slug),
9.5x faster kitchen-sink. See `migrating-from-velite.md`.

## Why do I see two `<figure>` levels?

You set `title="..."` on a code block. Without title, only `<pre>`.
With title, `<figure data-dmc-figure>` wraps `<figcaption
data-dmc-title>` + `<pre>`.

## How do I disable a built-in transformer?

Build with `--no-default-features` and add only the features you want.
Or override per-config (e.g. `markdown_gfm: false` disables GFM,
`math_engine: "mathml"` swaps math engine).

## Cache invalidation got it wrong. What now?

Wipe and rebuild:

```bash
rm -rf <output_dir>/.cache
dmc build
```

If a stale-cache repro is reliable, the cache fingerprint is missing
a config field. File a bug; in the meantime, bump the dmc package
version in `package.json` (the cache key includes the version).

## How big is the binary?

Slim (no default features): ~8 MB. Default features: ~15 MB. Most
weight is the syntect bundle (~5 MB) and KaTeX quick-js (~2 MB).
The dmc-napi `*.node` adds the Rust binary on top of any node
process; the build artefact ships once per platform.

## Can I run dmc without Node?

Yes. The Rust CLI `dmc build` runs end to end without Node. Foreign
plugins (sidecar) require Node; if your config has no foreign
plugins, the sidecar never spawns. dmc-napi is for JS consumers
(Next.js, Astro, etc) who want library-style access.

## Why does my JSX inside MDX render differently in HTML vs MDX body?

`HtmlEmitter` cannot run JS (no React runtime), so `JsxExpression`
nodes are dropped with `GW002 HtmlExpressionDropped`.
`MdxBodyEmitter` inlines them verbatim. For a static HTML site, use
`emit_html: true`. For a runtime React/MDX site, use
`emit_body: true` and feed `body` into `@mdx-js/react`.

## Does dmc support diff syntax in code blocks?

Not yet (`+`/`-` line markers, github diff style). Open lever for a
follow-up.

## Does dmc support inline code highlighting?

Not yet (`` `code{:rust}` `` style). Inline code renders without
syntax color. Open lever.

## Why is my ordered list re-numbering wrong?

CommonMark says ordered lists start at the first marker's number.
The dmc parser captures the first marker's number into `List.start`;
HTML emit produces `<ol start="...">` only when start != 1.
