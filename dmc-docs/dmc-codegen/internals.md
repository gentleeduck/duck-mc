# dmc-codegen internals

## Sink dispatch order

`Walker::walk` is one pre-order DFS. For each node visited:

1. `enter(node, ctx)` fires on every sink in slice order
2. recurse into `node`'s children
3. `leave(node, ctx)` fires on every sink in REVERSE slice order (LIFO)

LIFO leave matters when sinks need balanced push/pop. Wrap a
parent's emit in:

```rust
fn enter(&mut self, n: &Node, _ctx: &WalkCtx) {
    if let Node::Bold(_) = n { self.out.push_str("<strong>"); }
}
fn leave(&mut self, n: &Node, _ctx: &WalkCtx) {
    if let Node::Bold(_) = n { self.out.push_str("</strong>"); }
}
```

`enter` opens, `leave` closes. LIFO ensures inner closes before outer.

## Why one walker, three sinks

A naive design would walk the AST three times (once per output:
HTML, MDX body, accumulator). One walk is faster:

- ~3x fewer pointer chases
- shared `WalkCtx` (depth, index, parent) avoids redundant computes
- a sink that only cares about a few node types stays cheap

The trait method has a default `leave` impl so single-purpose sinks
can omit it.

## `WalkCtx`

```rust
pub struct WalkCtx<'a> {
    pub depth: usize,
    pub index: usize,
    pub parent: Option<&'a Node>,
}
```

Path: `dmc_codegen::WalkCtx`. Read-only; sinks never mutate it.

| field | meaning |
|-------|---------|
| `depth` | nesting count above the current node. Top-level child = 0 |
| `index` | position among parent's children (0-based) |
| `parent` | `None` when visiting a top-level child of the document |

`WalkCtx::root()` constructs the entry context. `child(parent, index)`
pushes one level deeper.

## Document is not a node event

`Walker::walk` iterates `doc.children` directly; no
`Node::Document` enter event. Sinks needing a document boundary
subscribe to `Frontmatter` (always emitted first when present) or to
their first node.

## Table walking

Tables hold `Vec<TableRow>` of `Vec<TableCell>`, neither of which is
a `Node`. The walker special-cases tables:

```rust
match node {
    Node::Table(t) => {
        for row in &t.children {
            for cell in &row.cells {
                for (i, kid) in cell.children.iter().enumerate() {
                    Self::walk_node(kid, &ctx.child(node, i), sinks);
                }
            }
        }
    }
    _ => {
        for (i, kid) in Node::children_of(node).iter().enumerate() {
            Self::walk_node(kid, &ctx.child(node, i), sinks);
        }
    }
}
```

Cells are walked inline; rows / cells never fire `enter` / `leave`
events themselves. Sinks emit `<table>` / `<tr>` / `<td>` from the
`Node::Table` enter / leave plus inline iteration over the inner
node tree.

## `Node::children_of`

```rust
pub fn children_of(node: &Node) -> &[Node];
pub fn children_of_mut(node: &mut Node) -> Option<&mut Vec<Node>>;
```

Variant lookup table. Returns `&[]` (or `None`) for leaf variants
(Text, InlineCode, CodeBlock, Image, JsxExpression, ...). Used by
both `Walker` and the mutating `walk_root` in dmc-transform.

## Diagnostics ownership

Each sink owns a private `DiagnosticEngine<Code>` during the walk:

```rust
let mut html_sink = HtmlEmitter::new();   // private engine
let mut body_sink = MdxBodyEmitter::new(); // private engine
Walker::new(&doc).walk(&mut [&mut html_sink, &mut body_sink]);

let (html, html_diag) = html_sink.into_parts();
let (body, body_diag) = body_sink.into_parts();
caller_engine.extend(html_diag);
caller_engine.extend(body_diag);
```

Avoids `RefCell` / lock contention on every emit. Sinks merge into
the caller's engine after the walk.

## Why no async

Codegen is bytes-in / bytes-out. No I/O. Strict CPU-bound path.
Async overhead would dominate.

## Allocations

`String::with_capacity(source.len())` is the typical preallocation
hint at sink construction. Output usually grows to ~2-3x source
size; the buffer extends naturally without much reallocation.

A 100-line MDX file compiles to HTML in ~50 us total walker + emit
time. Most of the cost lives in the `Pipeline` transformers, not
codegen.
