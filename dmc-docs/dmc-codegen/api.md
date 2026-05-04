# API

Every public item in `dmc-codegen`. Canonical paths in headings.

## `dmc_codegen::NodeSink`

```rust
pub trait NodeSink {
  fn enter(&mut self, node: &Node, ctx: &WalkCtx);
  fn leave(&mut self, _node: &Node, _ctx: &WalkCtx) {}
}
```

Callback pair fired by `Walker` at every node. `leave` defaults to a no-op so leaf-only sinks just override `enter`.

## `dmc_codegen::WalkCtx`

```rust
pub struct WalkCtx<'a> {
  pub depth: usize,            // ancestor count; top-level kids = 0
  pub index: usize,            // 0-based index in parent's children
  pub parent: Option<&'a Node>,// None for top-level kids of Document
}

impl<'a> WalkCtx<'a> {
  pub fn root() -> Self;
  pub fn child(&self, parent: &'a Node, index: usize) -> Self;
}
```

Read-only position info handed to every sink callback.

## `dmc_codegen::Walker`

```rust
pub struct Walker<'a> { /* private */ }

impl<'a> Walker<'a> {
  pub fn new(doc: &'a Document) -> Self;
  pub fn walk(self, sinks: &mut [&mut dyn NodeSink]);
}
```

Pre-order DFS over `doc.children`. At each node every sink's `enter` fires (slice order); the walker recurses into the node's children; finally every sink's `leave` fires (reverse slice order, LIFO).

`Document` is **not** surfaced as a `Node::Document` event - the walker iterates `doc.children` directly. Sinks needing a document boundary subscribe to the `Frontmatter` node or to their first node received.

`Table`, `TableRow`, `TableCell` are special: only `Table` is fanned out to sinks; rows and cells aren't `Node` variants the walker can surface, so emitters render the whole table inline on `enter Table`.

## `dmc_codegen::html::HtmlEmitter`

```rust
pub struct HtmlEmitter { /* private */ }

impl HtmlEmitter {
  pub fn new() -> Self;
  pub fn into_string(self) -> String;
  pub fn into_parts(self) -> (String, DiagnosticEngine<dmc_diagnostic::Code>);
  pub fn render(doc: &Document) -> (String, DiagnosticEngine<dmc_diagnostic::Code>);
}

impl Default for HtmlEmitter { /* same as new() */ }
impl NodeSink for HtmlEmitter { /* enter + leave */ }
```

Re-exported as `dmc_codegen::HtmlEmitter`.

- `new` - empty buffer + fresh diagnostic engine.
- `into_string` - pull the rendered HTML, drop the diag engine.
- `into_parts` - pull `(html, diag)`; caller merges diag via `outer.extend(diag)`.
- `render` - convenience: build emitter, drive the walker, return `(html, diag)`. Use when no other sink shares the walk.

## `dmc_codegen::html::render_html`

```rust
pub fn render_html(doc: &Document) -> String;
```

Convenience. Drives a one-sink walk, returns just the HTML string. Diagnostic engine is dropped.

## `dmc_codegen::mdx::MdxBodyEmitter`

```rust
pub struct MdxBodyEmitter { /* private */ }

impl MdxBodyEmitter {
  pub fn new() -> Self;
  pub fn into_string(self) -> String;
  pub fn into_parts(self) -> (String, DiagnosticEngine<dmc_diagnostic::Code>);
  pub fn render(doc: &Document) -> (String, DiagnosticEngine<dmc_diagnostic::Code>);
}

impl Default for MdxBodyEmitter { /* same as new() */ }
impl NodeSink for MdxBodyEmitter { /* enter + leave */ }
```

Re-exported as `dmc_codegen::MdxBodyEmitter`.

Emits a JS function body of the form:

```js
import ... // hoisted prelude
export ...
function _createMdxContent(props) {
  const _components = (props && props.components) || {};
  const { Fragment, jsx, jsxs } = arguments[0];
  return jsxs(Fragment, { children: [ /* nodes */ ] });
}
return _createMdxContent(arguments[0]);
```

- `new` - empty stack with a single root frame.
- `into_string` - assemble + return body string, drop diag.
- `into_parts` - assemble + return `(body, diag)`.
- `render` - convenience: build, walk, return `(body, diag)`.

## `dmc_codegen::mdx::render_mdx_body`

```rust
pub fn render_mdx_body(doc: &Document) -> String;
```

Convenience. Drives a one-sink walk, returns just the body string. Diagnostic engine is dropped.

## Re-exports at crate root

```rust
pub use html::{HtmlEmitter, render_html};
pub use mdx::{MdxBodyEmitter, render_mdx_body};
```

So `dmc_codegen::HtmlEmitter` etc. all resolve.
