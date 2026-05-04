# Compatibility

What dmc runs on, what versions of related ecosystems it speaks.

## Platforms (napi)

| platform | binary | status |
|----------|--------|--------|
| Linux x64 (glibc) | `dmc.linux-x64-gnu.node` | shipping |
| Linux x64 (musl) | `dmc.linux-x64-musl.node` | planned |
| Linux arm64 | `dmc.linux-arm64-gnu.node` | planned |
| macOS x64 | `dmc.darwin-x64.node` | planned |
| macOS arm64 | `dmc.darwin-arm64.node` | planned |
| Windows x64 | `dmc.win32-x64-msvc.node` | planned |

The napi-rs build script targets each in `napi.triples`; CI publishes
prebuilt binaries to npm. From source: `pnpm --filter @gentleduck/md run
build` produces a binary for the host platform.

## Node

| Node | status |
|------|--------|
| 20.x | tested, recommended |
| 22.x | tested |
| 18.x | should work; not actively tested |

Below 18 is unsupported (uses `for await` over stdin in dmc-sidecar
which requires a recent Node).

## Package manager

| pm | works |
|----|-------|
| pnpm | preferred (workspace setup) |
| npm | works for consumers |
| yarn | works for consumers |
| bun | works; preferred TS host for `.ts` configs |

## TS host for config

dmc routes `.ts` / `.js` / `.mjs` configs through:

1. `bun` (preferred; faster startup)
2. `node + tsx` (fallback)

One of the two must be on PATH. `.toml` configs avoid this.

## Frameworks

| framework | guide |
|-----------|-------|
| Next.js (App Router) | [`nextjs.md`](nextjs.md) |
| Astro | [`astro.md`](astro.md) |
| SvelteKit | [`sveltekit.md`](sveltekit.md) |
| Vite (any) | [`vite.md`](vite.md) |
| Remix | works the same as Next.js (load JSON in loader) |
| Solid Start | works the same as Vite |
| Webpack-based | works as long as `*.json` import works |

dmc is framework-agnostic: it writes JSON, you import the JSON. No
framework-specific runtime.

## CommonMark / GFM

| feature | dmc support | notes |
|---------|------------|-------|
| paragraphs | yes | |
| headings (ATX) | yes | up to h6; clamped from h7+ with `PW003` |
| headings (Setext) | partial | `===` / `---` underlines parse but uncommon paths may fail |
| blockquote | yes | nested supported via stack-based parser |
| nested blockquote with deeper levels | yes | |
| ordered list | yes | with `start=` |
| unordered list | yes | |
| nested list | yes | |
| loose list | yes | items wrapped in `<p>` |
| task list (GFM) | yes | `- [ ]` / `- [x]` |
| table (GFM) | yes | with column alignment |
| strikethrough (GFM) | yes | `~~text~~` |
| autolink (GFM) | yes | bare URLs in text |
| inline code | yes | |
| code block (fenced) | yes | with `lang` + `meta` |
| code block (indented) | yes | 4-space indent (disambiguated from nested list) |
| horizontal rule | yes | `---`, `***`, `___` |
| link | yes | with optional title |
| image | yes | with optional title |
| escape (`\*`, etc) | yes | unescaped at render time |
| triple emphasis | yes | `***x***` -> `<em><strong>x</strong></em>` |
| reference-style links | partial | basic `[text][ref]` works; complex cases may fail |

## MDX

| feature | dmc support | notes |
|---------|------------|-------|
| JSX self-closing | yes | `<Comp prop="v" />` |
| JSX element with children | yes | `<Comp>...</Comp>` |
| JSX fragment | yes | `<>...</>` |
| JSX expression | yes | `{expr}` (rendered into MDX body, dropped from HTML) |
| import / export | yes | top-level only |
| frontmatter (YAML) | yes | `---...---` block |

## unified plugins (sidecar)

| plugin | status |
|--------|--------|
| `remark-gfm` | stripped, native handles |
| `remark-math` | stripped when `math` feature on |
| `remark-emoji` | stripped when `emoji` feature on |
| `rehype-pretty-code` | stripped when `pretty-code` feature on |
| `shiki` | stripped (replaced by syntect) |
| `rehype-katex` / `rehype-mathjax` | stripped when `math` feature on |
| `rehype-slug` / `rehype-autolink-headings` | stripped (native handles) |
| any other unified plugin | runs in dmc-sidecar |

See [`plugins.md`](plugins.md).

## Math

| engine | output | speed |
|--------|--------|-------|
| `MathEngine::Katex` (default) | KaTeX HTML + accessibility MathML | 1-5 ms / expression |
| `MathEngine::Mathml` | bare `<math>` element | ~10 us / expression |

Both cached on disk (`<output>/.cache/math.json`).

## Themes (pretty-code)

20 bundled (Catppuccin variants, Nord, OneHalf, Solarized, gruvbox,
TwoDark, etc). Add custom via `dmc-highlight/assets/themes-bat/*.tmTheme`.

## Grammars (pretty-code)

~250 bundled (rust, ts, tsx, py, go, ruby, bash, json, yaml, etc).
Refresh from shiki via `scripts/convert-shiki-assets.mjs`.

## Schema

Zod-style. Subset of velite's `s` builder API.
[`../dmc-napi/schema-builder.md`](../dmc-napi/schema-builder.md)
lists supported methods.

## Deployment targets

dmc emits static JSON. Deploy anywhere that serves static files:
Vercel, Netlify, Cloudflare Pages, GitHub Pages, S3, etc.

The Rust binary is needed only at build time. Production servers do
not need Rust or Node-tsx (just a static-file host).

## Versioning

Pre-1.0. Schema may break on minor versions. Cache key includes
`CARGO_PKG_VERSION` so bumps auto-invalidate. Pin to a known version
in your `package.json`.

## Out of scope

| feature | status |
|---------|--------|
| Browser-side dmc | no (Rust napi binary; node-only) |
| Deno | no (npm package via npm: prefix may work; not tested) |
| Bun runtime | works as TS host for configs; bun's own MDX runtime is separate |
