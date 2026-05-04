# dmc-codegen examples

## Render a paragraph

```rust
use dmc_codegen::render_html;
use dmc_parser::parse;

let html = render_html(&parse("hello **world**\n"));
assert!(html.contains("<p>hello <strong>world</strong></p>"));
```

`render_html` is a convenience that spawns its own `Walker` and
`HtmlEmitter`.

## Render a heading with auto-anchor

```rust
use dmc_codegen::render_html;
use dmc_parser::parse;
use dmc_transform::{AutolinkHeadings, Pipeline};

let mut doc = parse("# Hello World\n");
Pipeline::new()
    .add(AutolinkHeadings::new())
    .run_silent(&mut doc);

let html = render_html(&doc);
assert!(html.contains(r#"<h1 id="hello-world">"#));
assert!(html.contains(r#"<a href="#hello-world" class="subheading-anchor""#));
```

`AutolinkHeadings` mutates the AST so the heading already contains
the anchor wrap by the time `HtmlEmitter` runs.

## Render a code block via PrettyCode

```rust
#[cfg(feature = "pretty-code")]
{
    use dmc_codegen::render_html;
    use dmc_parser::parse;
    use dmc_transform::{Pipeline, PrettyCode};

    let mut doc = parse("```rust title=\"lib.rs\"\nfn main() {}\n```\n");
    Pipeline::new()
        .add(PrettyCode::default())
        .run_silent(&mut doc);

    let html = render_html(&doc);
    assert!(html.contains(r#"<figure data-dmc-figure"#));
    assert!(html.contains(r#"<figcaption data-dmc-title"#));
    assert!(html.contains(">lib.rs<"));
    assert!(html.contains(r#"<pre data-language="rust""#));
}
```

`PrettyCode` rewrites `Node::CodeBlock` to a JSX subtree; the html
emitter renders it like any other JsxElement.

## MDX body output

```rust
use dmc_codegen::render_mdx_body;
use dmc_parser::parse;

let body = render_mdx_body(&parse("# hi\n\n*world*"));
assert!(body.contains("function _createMdxContent"));
assert!(body.contains("\"h1\""));
```

Wrap with imports / exports via `wrap_mdx_module(&body, &imports)`
when targeting full-module output.

## Custom sink

```rust
use dmc_codegen::{NodeSink, WalkCtx, Walker};
use dmc_parser::ast::*;
use dmc_parser::parse;

struct Counter(usize);

impl NodeSink for Counter {
    fn enter(&mut self, _node: &Node, _ctx: &WalkCtx) {
        self.0 += 1;
    }
}

let doc = parse("# hi\n\nworld\n");
let mut counter = Counter(0);
Walker::new(&doc).walk(&mut [&mut counter]);
println!("nodes visited: {}", counter.0);
```

`NodeSink` has only two methods (`enter`, optional `leave`). Plug
into the same `Walker` that drives the built-in emitters.
