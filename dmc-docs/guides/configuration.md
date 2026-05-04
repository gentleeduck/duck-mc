# Configuration

dmc reads `duck-md.config.ts` (or `.js`, `.mjs`, `.toml`). The TS API
is type-checked via `defineConfig`.

## Top level

```ts
defineConfig({
  root: "content",
  output: {
    data: ".gentleduck",
    assets: "public/assets",
    base: "/assets/",
    name: "[name]-[hash:6].[ext]",
    clean: false,
    format: "esm",
    html: true,
  },
  clean: false,
  strict: false,
  cacheEnabled: true,
  collections: { /* ... */ },
  markdown: { /* ... */ },
  mdx: { /* ... */ },
  prepare(data) { /* ... */ },
  complete(data) { /* ... */ },
  loaders: [/* ... */],
});
```

| field | default | use |
|-------|---------|-----|
| `root` | required | content root |
| `output.data` | `.gentleduck` | output dir |
| `output.assets` | none | copy_linked_files target |
| `output.base` | none | public URL prefix for copied assets |
| `output.name` | `[name]-[hash:6].[ext]` | asset filename pattern |
| `output.html` | false | include rendered HTML field on records |
| `output.format` | `esm` | index.js style |
| `clean` | false | wipe output dir before build |
| `strict` | false | fail on warning (caller-driven) |
| `cacheEnabled` | true | persistent file + math cache |

## Collections

```ts
collections: {
  docs: {
    name: "doc",                 // output filename stem
    pattern: "docs/**/*.mdx",    // glob relative to root
    schema: s.object({/*...*/}), // Zod frontmatter validator
    single: false,               // emit one record (object) vs array
  },
}
```

`name` becomes `<output_dir>/<name>.json` and the export key in
`index.js`.

## `markdown` and `mdx`

```ts
markdown: {
  gfm: true,                  // GFM in dmc parser
  removeComments: false,
  copyLinkedFiles: false,     // copy referenced files to output.assets
  remarkPlugins: [],          // foreign plugins for the sidecar
  rehypePlugins: [],          // foreign plugins for the sidecar
}

mdx: {
  outputFormat: "function-body" | "module",
  minify: false,
  remarkPlugins: [],
  rehypePlugins: [],
}
```

Plugins owned by native dmc transformers are stripped from these lists
before the sidecar is invoked. Native owners:

- `remark-gfm`
- `remark-math`
- `remark-emoji`
- `rehype-pretty-code`
- `shiki`
- `rehype-katex`
- `rehype-mathjax`
- `rehype-slug`
- `rehype-autolink-headings`

If the gate strips every plugin, the sidecar is never spawned.

## `prettyCode`

Override theme bundle:

```ts
defineConfig({
  prettyCode: {
    theme: {
      light: "Catppuccin Latte",
      dark: "Catppuccin Mocha",
    },
    defaultMode: "dark",
  },
});
```

Or single theme:

```ts
prettyCode: { theme: "Nord" }
```

See `dmc-docs/dmc-transform/transformers/pretty-code.md`.

## `mathEngine`

```ts
mathEngine: "katex"   // default; KaTeX HTML, slow but visual parity with rehype-katex
mathEngine: "mathml"  // pulldown-latex MathML, fast, plainer visual
```

Saves ~300 ms on a 1000-file kitchen sink at the cost of native browser
MathML rendering instead of KaTeX layout.

## TOML alternative

```toml
root = "content"

[output]
data = ".gentleduck"
html = true

[[collections]]
name = "doc"
pattern = "docs/**/*.mdx"
```

Functions and Pluggable refs are not expressible in TOML; use TS for
those.

## Caching

```ts
cacheEnabled: true  // default
```

Wipe via `rm -rf <output>/.cache`. See
`dmc-docs/architecture/caching.md`.
