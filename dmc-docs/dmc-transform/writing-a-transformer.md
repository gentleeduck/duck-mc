# Writing a transformer

A transformer is a struct implementing `Transformer`. It mutates a
`Document` and emits diagnostics. Most have a sibling `Visitor` impl
that does the actual work.

## Skeleton

```rust
use dmc_diagnostic::Code;
use dmc_diagnostic::metadata::SourceMeta;
use dmc_parser::ast::*;
use dmc_transform::{NodeAction, Visitor, walk_root, Transformer};
use duck_diagnostic::DiagnosticEngine;

pub struct MyPass;

impl Transformer for MyPass {
    fn name(&self) -> &str { "my-pass" }

    fn transform(
        &self,
        doc: &mut Document,
        _meta: &SourceMeta,
        _engine: &mut DiagnosticEngine<Code>,
    ) {
        let mut v = Apply;
        walk_root(&mut doc.children, &mut v);
    }
}

struct Apply;

impl Visitor for Apply {
    fn visit_node(&mut self, node: &mut Node) -> NodeAction {
        // inspect / mutate
        NodeAction::Keep
    }
}
```

## With config

```rust
pub struct MyPass {
    pub theme: String,
}

impl Transformer for MyPass {
    fn name(&self) -> &str { "my-pass" }
    fn transform(&self, doc: &mut Document, _meta, _engine) {
        let mut v = Apply { theme: &self.theme };
        walk_root(&mut doc.children, &mut v);
    }
}

struct Apply<'a> {
    theme: &'a str,
}

impl<'a> Visitor for Apply<'a> { /* ... */ }
```

Borrow the config into the visitor; lifetime tied to the transformer.

## Replacing a node

```rust
impl Visitor for Apply {
    fn visit_node(&mut self, node: &mut Node) -> NodeAction {
        let Node::CodeBlock(cb) = node else { return NodeAction::Keep };
        if cb.lang.as_deref() != Some("mermaid") {
            return NodeAction::Keep;
        }
        let span = cb.span.clone();
        let svg = render_mermaid(&cb.value);
        NodeAction::Replace(vec![Node::JsxSelfClosing(JsxSelfClosing {
            name: "MermaidSvg".into(),
            attrs: vec![JsxAttr {
                name: "svg".into(),
                value: JsxAttrValue::String(svg),
                span: span.clone(),
            }],
            span,
        })])
    }
}
```

Replacement nodes are not re-visited; safe for transformers that
produce nodes that look like their own input pattern.

## Removing a node

```rust
fn visit_node(&mut self, node: &mut Node) -> NodeAction {
    if let Node::Image(i) = node && i.src.starts_with("javascript:") {
        return NodeAction::Remove;
    }
    NodeAction::Keep
}
```

## Diagnostics

```rust
use duck_diagnostic::{Diagnostic, Label};

self.pending.push(
    Diagnostic::new(Code::ImportFileNotFound, "code-import: file not on disk")
        .with_label(Label::primary(span, Some("requested here".into())))
        .with_help("check the file= attr"),
);
```

Visitors that need to emit diagnostics carry a `Vec<Diagnostic<Code>>`
on the visitor struct, then drain into the transformer's `engine`
parameter after `walk_root` returns:

```rust
fn transform(&self, doc, _meta, engine) {
    let mut v = Apply { pending: Vec::new() };
    walk_root(&mut doc.children, &mut v);
    for d in v.pending.drain(..) {
        engine.emit(d);
    }
}
```

This pattern is what `Mermaid` does.

## Threading + sharing state

`Pipeline` boxes transformers as `dyn Transformer + Send + Sync`. So:

- impl `Send + Sync` on the transformer struct
- `transform` takes `&self`, not `&mut self`

For mutable shared state (e.g. caches), use `Mutex<...>`.

## Registering

For one-off use:

```rust
let pipeline = Pipeline::with_defaults_for(&cfg).add(MyPass);
```

For built-in inclusion: add to `Pipeline::with_defaults_for` in
`dmc-transform/src/pipeline.rs` under a Cargo feature flag, mirror in
`dmc-transform/Cargo.toml`. See `dmc-docs/dmc-transform/transformers/`
for the existing built-ins as reference.

## Testing

```rust
use dmc_parser::parse;
use dmc_transform::Pipeline;

#[test]
fn drops_javascript_images() {
    let mut doc = parse("![bad](javascript:alert(1))");
    Pipeline::new().add(MyPass).run_silent(&mut doc);
    assert!(!doc.children.iter().any(|n| matches!(n, Node::Image(_))));
}
```

`run_silent` is the test helper; uses synthetic meta + throwaway
engine.
