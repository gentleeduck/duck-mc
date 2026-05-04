# Plugin recipes

Common foreign plugins that ship with dmc-sidecar today and how to
wire them into your config.

## remark-frontmatter

Already built in (dmc parses `---...---` frontmatter natively). Listed
here for completeness; do NOT add to config.

## remark-toc

Generates a `[TOC]` placeholder in the rendered HTML.

```ts
import remarkToc from "remark-toc";

defineConfig({
  markdown: {
    remarkPlugins: [[remarkToc, { tight: true, maxDepth: 3 }]],
  },
});
```

dmc's Accumulator already produces a structured `toc` field on every
record. Use `record.toc` instead unless you need an inline `[TOC]`
placeholder.

## remark-directive

Custom directive syntax (`:::warning`).

```ts
import remarkDirective from "remark-directive";

defineConfig({
  markdown: {
    remarkPlugins: [remarkDirective],
  },
});
```

## remark-breaks

Render every soft break as `<br>` (twitter-style).

```ts
import remarkBreaks from "remark-breaks";

defineConfig({
  markdown: {
    remarkPlugins: [remarkBreaks],
  },
});
```

## remark-smartypants

Curly quotes, em-dashes, ellipses (the "smartypants" transformation).

```ts
import remarkSmartypants from "remark-smartypants";

defineConfig({
  markdown: {
    remarkPlugins: [remarkSmartypants],
  },
});
```

Note: dmc's docs style avoids these characters. Use only on
user-facing prose, never on technical content.

## remark-pdf-link / remark-images

Wrap `<img>` in `<picture>` with size hints.

```ts
import remarkImages from "remark-images";

defineConfig({
  markdown: {
    remarkPlugins: [remarkImages],
  },
});
```

dmc's `copy-linked-files` can handle the asset side; pair the two.

## rehype-external-links

Add `rel="nofollow noopener"` and `target="_blank"` to outbound
links.

```ts
import rehypeExternalLinks from "rehype-external-links";

defineConfig({
  markdown: {
    rehypePlugins: [
      [rehypeExternalLinks, { target: "_blank", rel: ["nofollow", "noopener"] }],
    ],
  },
});
```

## rehype-sanitize

Sanitise HTML (drops `<script>`, restricts attrs to a safe set).

```ts
import rehypeSanitize from "rehype-sanitize";

defineConfig({
  markdown: {
    rehypePlugins: [rehypeSanitize],
  },
});
```

USE IF input is untrusted. dmc preserves authored MDX verbatim;
sanitisation is downstream's responsibility.

## rehype-figure

Wrap images with captions in `<figure>` + `<figcaption>`.

```ts
import rehypeFigure from "rehype-figure";

defineConfig({
  markdown: {
    rehypePlugins: [rehypeFigure],
  },
});
```

## rehype-citation

Render bibliographic citations.

```ts
import rehypeCitation from "rehype-citation";

defineConfig({
  markdown: {
    rehypePlugins: [
      [rehypeCitation, { bibliography: "./refs.bib", csl: "vancouver" }],
    ],
  },
});
```

## rehype-mermaid (alt to native Mermaid)

If you do not have `mmdc` on PATH, use the JS plugin:

```ts
import rehypeMermaid from "rehype-mermaid";

defineConfig({
  markdown: {
    rehypePlugins: [rehypeMermaid],
  },
});
```

dmc's native `Mermaid` transformer produces the same output via
`mmdc`. Use one or the other; not both.

## Stripped automatically

Listed for completeness; the plugin gate skips these so the sidecar
never sees them:

| name | replacement |
|------|-------------|
| `remark-gfm` | dmc parser native GFM |
| `remark-math` | `math` transformer |
| `remark-emoji` | `emoji` transformer |
| `rehype-pretty-code` | `pretty-code` transformer |
| `shiki` | syntect (`pretty-code`) |
| `rehype-katex` | `math` transformer (KaTeX engine) |
| `rehype-mathjax` | `math` transformer |
| `rehype-slug` | `autolink-headings` transformer |
| `rehype-autolink-headings` | `autolink-headings` transformer |

If you list one of these and it does NOT get stripped, the
corresponding feature is off at compile time. Rebuild with the
feature on, or the plugin will run twice (once native, once sidecar).

## Order matters

Plugins run in the order listed. Rule of thumb:

1. Source mutations: remark plugins (`remark-directive`,
   `remark-frontmatter`, etc).
2. AST -> hAST conversion: handled by unified internally.
3. HTML mutations: rehype plugins (`rehype-sanitize`,
   `rehype-external-links`).
4. Stringify: handled by unified internally.

Dmc-sidecar wires `remark-parse -> remark-rehype -> rehype-stringify`
with user-supplied plugins inserted before each conversion.

## Testing a plugin

```bash
node dmc-sidecar/index.mjs <<EOF
{"id":1,"markdown":"# hi","remarkPlugins":[],"rehypePlugins":[["rehype-external-links",{"target":"_blank"}]]}
EOF
```

Returns one line of JSON with the rendered HTML. Inspect to confirm
the plugin took effect.

## Performance

Each foreign plugin adds:

- one-time plugin import (cached in the sidecar's per-spec
  processor cache after first call)
- per-file transform (varies wildly per plugin; profile)
- one IPC round-trip per file (~200 us, batched in future)

Heavy plugins (rehype-citation, rehype-mermaid, rehype-prism) cost
1-5 ms per file. Light plugins (rehype-external-links) cost <100 us.

Bench the chain end-to-end via `cargo run --release --features
pretty-code --example bench` after each plugin addition.
