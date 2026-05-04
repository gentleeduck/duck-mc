# Visitor

The `Visitor` trait + `walk_root` driver are how transformers mutate
the AST.

## Trait

```rust
pub trait Visitor {
    fn visit_node(&mut self, _node: &mut Node) -> NodeAction {
        NodeAction::Keep
    }
}
```

Path: `dmc_transform::Visitor`. Default impl keeps every node and
recurses; override only the variants you care about.

## `NodeAction`

```rust
pub enum NodeAction {
    Keep,
    KeepSkipChildren,
    Replace(Vec<Node>),
    Remove,
}
```

Path: `dmc_transform::NodeAction`.

| variant | semantics |
|---------|-----------|
| `Keep` | keep node, recurse into its children |
| `KeepSkipChildren` | keep node, do not recurse |
| `Replace(vs)` | splice `vs` into the parent's `children` at the current index; replacements are not re-visited (avoids infinite loops on transformers that produce nodes matching their own pattern) |
| `Remove` | drop this node from the parent's children |

## Drivers

### `walk_root`

```rust
pub fn walk_root<V: Visitor>(children: &mut Vec<Node>, v: &mut V);
```

Path: `dmc_transform::walk_root`. Drives the visitor over a `Vec<Node>`
honoring every `NodeAction`. Most transformers call this on
`doc.children`.

### `walk_children_mut`

```rust
pub fn walk_children_mut<V: Visitor>(parent: &mut Node, v: &mut V);
```

Recurse into the per-variant inner children of `parent`. Leaf
variants (Text, InlineCode, CodeBlock, Image, JsxExpression, ...) are
no-ops. Tables get special treatment (rows + cells).

## Replace semantics

```rust
NodeAction::Replace(new) => {
    let n = new.len();
    children.splice(i..=i, new);
    i += n;
}
```

The replacement nodes are NOT re-visited in the same walk. They are
recursed into on the next outer pass if `Keep` is returned for them
later. Prevents transformers like `Math::Apply` from infinite-looping
on their own output.

## Custom visitor example

```rust
use dmc_transform::{NodeAction, Visitor, walk_root, Pipeline, Transformer};
use dmc_parser::ast::*;

struct DropImages;

impl Visitor for DropImages {
    fn visit_node(&mut self, node: &mut Node) -> NodeAction {
        match node {
            Node::Image(_) => NodeAction::Remove,
            _ => NodeAction::Keep,
        }
    }
}

struct DropImagesPass;
impl Transformer for DropImagesPass {
    fn name(&self) -> &str { "drop-images" }
    fn transform(&self, doc: &mut Document, _meta, _diag) {
        walk_root(&mut doc.children, &mut DropImages);
    }
}

let pipeline = Pipeline::new().add(DropImagesPass);
```

## Order details

| direction | order |
|-----------|-------|
| `enter` (visit_node) | parent before children |
| children iteration | slice order (low index first) |
| recursion | depth-first |

When a visitor mutates a node's children mid-iteration, indices are
managed by `walk_root` (`splice` on Replace, `remove` on Remove,
`i += 1` otherwise).
