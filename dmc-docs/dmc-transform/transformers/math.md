# `math`

Renders `$...$` (inline) and `$$...$$` (display) LaTeX expressions to one
of two backends - KaTeX HTML (default, exact `rehype-katex` parity) or
pulldown-latex MathML (fast, plainer visuals).

- **Source:** `dmc-transform/src/builtin/math.rs`
- **Feature flag:** `math`
- **Config struct:** [`MathEngine`](../src/config.rs)
- **TS slot:** `mdx.math`

## Configuration

```ts
import { defineConfig } from '@gentleduck/md/config'

export default defineConfig({
  mdx: {
    math: 'katex',     // 'katex' (default) | 'mathml'
  },
})
```

## Engines

| Engine | Speed | Output | Notes |
|---|---|---|---|
| `katex` | 1-5 ms / expr | HTML + KaTeX classes | Exact byte-for-byte match with `rehype-katex`. Pair with `katex.min.css`. |
| `mathml` | us / expr | MathML elements | Fast. Browser MathML rendering is functional but visually plainer than KaTeX HTML. |

## Source-level preprocess

Math runs in two phases:

1. **Pre-parse rewrite** - `$...$` / `$$...$$` get replaced with `<MathMl>`
   JSX wrappers _before_ the parser sees them. Otherwise `_` and `^`
   inside math would trigger Markdown emphasis / superscript handling.
2. **Pipeline pass** - the JSX `<MathMl>` nodes get rendered to the
   selected backend's output during the transform stage.

## Sidecar opt-out

Add any of `"remark-math"`, `"rehype-katex"`, `"rehype-mathjax"` to
`markdown.preferSidecar` to drop the native pass and defer to the JS
plugin chain.
