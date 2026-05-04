# Walker

Pre-order DFS. Single traversal, fans out to every active `NodeSink`.

## Order

For each node:

1. Every sink's `enter` fires, in **slice order** (sink 0, then sink 1, ...).
2. Walker recurses into the node's children (each becomes its own enter / recurse / leave cycle).
3. Every sink's `leave` fires, in **reverse slice order**, LIFO.

```text
   walk(&mut [&mut a, &mut b])
       a.enter(node)
       b.enter(node)
           ... walk children ...
       b.leave(node)
       a.leave(node)
```

A sink that pushes state on `enter` and pops on `leave` will see a balanced bracket sequence even when other sinks are attached.

## Document is not a Node event

The walker iterates `doc.children` directly. There is **no** `Node::Document` enter / leave event. Sinks needing a document boundary either:

- Subscribe to the first node they receive (e.g. flush a prelude).
- Or watch for `Node::Frontmatter`, which comes first when present.

```rust
pub struct Walker<'a> { doc: &'a Document }

impl<'a> Walker<'a> {
  pub fn new(doc: &'a Document) -> Self;
  pub fn walk(self, sinks: &mut [&mut dyn NodeSink]);
}
```

## `WalkCtx`

```rust
pub struct WalkCtx<'a> {
  pub depth: usize,             // 0 = top-level child of Document
  pub index: usize,             // 0-based among parent's children
  pub parent: Option<&'a Node>, // None for top-level kids
}
```

Position info handed to every callback. Read-only. Constructed by the walker via `WalkCtx::root()` for the top-level frame and `ctx.child(parent, i)` for each descent step.

## Children source

For most nodes: `Node::children_of(node)` (defined on `dmc_parser::ast::Node`).

`Table` is the exception - its children are `TableRow` / `TableCell`, which are not walker-visible `Node` variants. Walker descends into rows/cells but only to find nested `Node`s; emitters typically use `in_table_depth` to suppress redundant events on cell content (they render the whole `<table>` up front on `enter Table`).

## Multi-sink example

```rust
use dmc_codegen::{HtmlEmitter, MdxBodyEmitter, Walker};
use dmc_parser::parse;

let doc = parse("# H\n\nbody");
let mut html = HtmlEmitter::new();
let mut mdx  = MdxBodyEmitter::new();

Walker::new(&doc).walk(&mut [&mut html, &mut mdx]);

let (h, h_diag) = html.into_parts();
let (m, m_diag) = mdx.into_parts();
```

One DFS. Two outputs. No second pass over the AST.
