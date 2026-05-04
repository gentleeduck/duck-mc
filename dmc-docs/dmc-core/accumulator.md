# Accumulator

Third sink alongside `HtmlEmitter` and `MdxBodyEmitter`. Pulls
metadata off the AST during the same `Walker` pass: frontmatter,
imports, exports, excerpt, word count, TOC.

Path: `dmc::engine::accumlator::Accumulator`.

## Struct

```rust
pub struct Accumulator {
    pub frontmatter: serde_json::Value,
    pub frontmatter_raw: String,
    pub imports: Vec<String>,
    pub exports: Vec<String>,
    pub plain: String,                       // text for excerpt + word count
    pub toc_flat: Vec<(u8, String, String)>, // (level, title, slug)
}
```

`toc_flat` is captured pre-nest; `into_compile_output` builds the
nested `Vec<TocItem>` shape.

## NodeSink impl

```rust
impl NodeSink for Accumulator {
    fn enter(&mut self, node: &Node, _ctx: &WalkCtx) { /* ... */ }
    fn leave(&mut self, node: &Node, _ctx: &WalkCtx) { /* ... */ }
}
```

`enter` collects:

- `Node::Frontmatter` -> parse YAML into `frontmatter`, keep raw
- `Node::Import / Export` -> push raw lines
- `Node::Heading(h)` -> `(level, plain_text(children), h.slug())`
- `Node::Text(t)` -> append to `plain`

`leave` is currently empty (frontmatter already captured on enter).

## Output shape

```rust
pub fn into_compile_output(
    self,
    source: &str,
    html: String,
    body: String,
    _cfg: &CompileConfig,
) -> CompileOutput;
```

Builds the final `CompileOutput`:

- `content` = source minus frontmatter (raw markdown body)
- `excerpt` = first 200 chars of plain text (sentence-bounded)
- `metadata` = `{ reading_time, word_count }` from plain
- `toc` = nested via level matching
- `frontmatter` / `frontmatter_raw` = captured during walk
- `imports` / `exports` = captured during walk
- `html` / `body` = passed in from the other sinks

## Excerpt

```rust
fn excerpt(plain: &str, max: usize) -> String {
    let mut out = String::new();
    for word in plain.split_whitespace() {
        if out.len() + word.len() + 1 > max { break }
        if !out.is_empty() { out.push(' '); }
        out.push_str(word);
    }
    out
}
```

Word-bounded; never splits mid-word. `max = 200` chars by default.

## Reading time

```rust
fn metadata(plain: &str) -> Metadata {
    let words = plain.split_whitespace().count() as u32;
    let reading_time = ((words as f32) / 200.0).ceil().max(1.0) as u32;
    Metadata { reading_time, word_count: words }
}
```

200 words/min. Minimum 1 minute.

## TOC nesting

```rust
fn toc(items: &[(u8, String, String)]) -> Vec<TocItem>;
```

Walks the flat list of `(level, title, slug)` tuples. Each heading
becomes a `TocItem`; deeper-level items become children of the
previous shallower one. Skipped levels (e.g. h1 -> h3 with no h2)
attach to the deepest open ancestor.

## When this sink runs

Always. `Compiler::compile_with_pipeline` constructs an `Accumulator`
unconditionally; `HtmlEmitter` and `MdxBodyEmitter` are gated on
`emit_html` / `emit_body` flags.

## Example

For source:

```mdx
---
title: Hello
---

# H1

intro paragraph

## H2

body
```

After Walker pass:

```rust
acc.frontmatter      = json!({ "title": "Hello" });
acc.frontmatter_raw  = "title: Hello\n";
acc.plain            = "H1 intro paragraph H2 body";
acc.toc_flat         = vec![(1, "H1".into(), "h1".into()), (2, "H2".into(), "h2".into())];
```

`into_compile_output` then produces:

```rust
CompileOutput {
    excerpt: "H1 intro paragraph H2 body",
    metadata: Metadata { reading_time: 1, word_count: 5 },
    toc: vec![
        TocItem { title: "H1", url: "#h1", items: vec![
            TocItem { title: "H2", url: "#h2", items: vec![] }
        ]}
    ],
    // ...
}
```
