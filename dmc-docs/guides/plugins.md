# Plugins

Two kinds of plugins: native (Rust transformers) and foreign (JS
unified plugins, run via dmc-sidecar).

## Native (default)

Listed in `Pipeline::with_defaults_for(cfg)`:

| transformer | feature flag | absorbs |
|-------------|--------------|---------|
| `CodeImport` | always on | reads `file=` attrs in code blocks |
| `BareUrlAutolink` | always on | wraps bare `https://...` in `<a>` |
| `AutolinkHeadings` | always on | heading slug + anchor wrap; replaces rehype-slug + rehype-autolink-headings |
| `DisableGfm` | always on (config-gated) | strips tables when gfm = false |
| `NpmCommand` | `npm-command` | `npm` -> tabs for pnpm/yarn/bun |
| `Mermaid` | `mermaid` | code lang=mermaid -> SVG via `mmdc` |
| `Emoji` | `emoji` | `:shortcode:` -> unicode; replaces remark-emoji |
| `Math` | `math` | `$...$` / `$$...$$` -> KaTeX or MathML; replaces remark-math + rehype-katex |
| `PrettyCode` | `pretty-code` | code blocks -> `<figure>` + spans; replaces rehype-pretty-code + shiki |
| `CopyLinkedFiles` | `assets` | copy local refs to output.assets |

## Foreign (sidecar)

Listed in `markdown.remarkPlugins` / `markdown.rehypePlugins` of the
config. Run by dmc-sidecar in a Node child.

```ts
import remarkFrontmatter from "remark-frontmatter";
import rehypeExternalLinks from "rehype-external-links";

defineConfig({
  markdown: {
    remarkPlugins: [remarkFrontmatter],
    rehypePlugins: [
      [rehypeExternalLinks, { rel: ["nofollow"] }],
    ],
  },
  // ...
});
```

## Gate

Before the sidecar is spawned, `CompileConfig::has_js_plugins` strips
every plugin owned by a native transformer. Stripped names:

- `remark-gfm`
- `remark-math`
- `remark-emoji`
- `rehype-pretty-code`
- `shiki`
- `rehype-katex`
- `rehype-mathjax`
- `rehype-slug`
- `rehype-autolink-headings`

If the stripped lists are empty, the sidecar is never spawned. Big
perf win when the user lists only those names.

## Writing a native transformer

```rust
use dmc_transform::{Pipeline, Transformer, NodeAction, Visitor, walk_root};
use dmc_parser::ast::*;

struct UppercaseHeadings;

impl Transformer for UppercaseHeadings {
    fn name(&self) -> &str { "uppercase-headings" }
    fn transform(&self, doc: &mut Document, _meta, _diag) {
        let mut v = Apply;
        walk_root(&mut doc.children, &mut v);
    }
}

struct Apply;
impl Visitor for Apply {
    fn visit_node(&mut self, node: &mut Node) -> NodeAction {
        if let Node::Heading(h) = node {
            for c in &mut h.children {
                if let Node::Text(t) = c {
                    t.value = t.value.to_uppercase();
                }
            }
        }
        NodeAction::Keep
    }
}

let pipeline = Pipeline::with_defaults_for(&cfg).add(UppercaseHeadings);
```

See `dmc-docs/dmc-transform/writing-a-transformer.md` for the full
walkthrough.

## Choosing native vs foreign

| if you want | choose |
|-------------|--------|
| max speed, you control the plugin | native |
| existing JS plugin (battle-tested) | foreign via sidecar |
| KaTeX-style math | native (KaTeX engine) or foreign rehype-katex (gate strips foreign) |
| niche unified plugin (no Rust port) | foreign |

Native passes happen in process; foreign passes incur one Node IPC
round-trip per file (or per batch in future).
