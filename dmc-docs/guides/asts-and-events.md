# AST + walker events

Reference for the `Node` enum and the events `NodeSink` sees during
the walker pass.

## Node enum

Path: `dmc_parser::ast::Node`. Every variant carries a `span:
duck_diagnostic::Span` field.

| variant | shape | when |
|---------|-------|------|
| `Document` | `{ children, span }` | root |
| `Frontmatter` | `{ raw, span }` | top-of-file YAML block |
| `Import` | `{ raw, span }` | top-level `import ...` |
| `Export` | `{ raw, span }` | top-level `export ...` |
| `Heading` | `{ level, children, span }` | `# ... ######` |
| `Paragraph` | `{ children, span }` | text body |
| `Text` | `{ value, span }` | plain text node |
| `Bold` | `Inline` | `**...**` / `__...__` |
| `Italic` | `Inline` | `*...*` / `_..._` |
| `Strikethrough` | `Inline` | `~~...~~` |
| `InlineCode` | `{ value, span }` | `` `code` `` |
| `CodeBlock` | `{ lang, meta, value, span }` | fenced code block |
| `Link` | `{ href, title, children, span }` | `[text](href)` |
| `Image` | `{ src, alt, title, span }` | `![alt](src)` |
| `HorizontalRule` | `{ span }` | `---` / `***` / `___` |
| `Blockquote` | `{ children, span }` | `> ...` |
| `List` | `{ ordered, start, children, span }` | `-` / `1.` lists |
| `ListItem` | `{ children, span }` | one item |
| `TaskListItem` | `{ checked, children, span }` | `- [ ]` / `- [x]` |
| `Table` | `{ align, children, span }` | GFM table |
| `TableRow` | `{ cells, span }` | NOT a Node child variant |
| `TableCell` | `{ children, span }` | NOT a Node child variant |
| `JsxElement` | `{ name, attrs, children, span }` | `<Comp>...</Comp>` |
| `JsxSelfClosing` | `{ name, attrs, span }` | `<Comp/>` |
| `JsxFragment` | `{ children, span }` | `<>...</>` |
| `JsxExpression` | `{ value, span }` | `{expr}` |
| `HardBreak` | `{ span }` | blank line between blocks |
| `SoftBreak` | `{ span }` | single newline within a block |

## Inline-only

```rust
pub struct Inline {
    pub children: Vec<Node>,
    pub span: Span,
}
```

Used by `Bold` / `Italic` / `Strikethrough`. Children are inline
nodes (Text, InlineCode, nested Bold/Italic/Strike, Link, etc).

## Children of helpers

```rust
impl Node {
    pub fn children_of(node: &Node) -> &[Node];
    pub fn children_of_mut(node: &mut Node) -> Option<&mut Vec<Node>>;
}
```

Returns the inner `children` vec for variants that have one. Leaf
variants (`Text`, `InlineCode`, `CodeBlock`, `Image`, `HorizontalRule`,
`HardBreak`, `SoftBreak`, `JsxExpression`) return `&[]` / `None`.

`TableRow` / `TableCell` are NOT `Node` variants; the walker
iterates them inline (see below).

## Walker events

`Walker::walk(sinks)` is one pre-order DFS over `doc.children`. For
each node visited:

```rust
for sink in sinks { sink.enter(node, ctx); }       // slice order
walk_children(node);                               // recurse
for sink in sinks.rev() { sink.leave(node, ctx); } // LIFO
```

`Document` itself is NOT surfaced as a `Node::Document` event; the
walker iterates `doc.children` directly.

`Table` walks its rows / cells inline:

```rust
Node::Table(t) => {
    for row in &t.children {
        for cell in &row.cells {
            for (i, kid) in cell.children.iter().enumerate() {
                walk_node(kid, &ctx.child(node, i), sinks);
            }
        }
    }
}
```

`TableRow` / `TableCell` never fire `enter` / `leave`.

## Span

```rust
pub struct Span {
    pub file: Arc<str>,
    pub line: usize,
    pub column: usize,
    pub length: usize,
}
```

Path: `duck_diagnostic::Span`. `file` shared via `Arc` so spans are
cheap to clone. Lines + columns are 1-based; `length` is byte count.

## When children mean different things

| variant | children semantics |
|---------|-------------------|
| `Heading` | inline content of the heading |
| `Paragraph` | inline content of the paragraph |
| `Bold` / `Italic` / `Strikethrough` | inline content inside the wrapper |
| `Blockquote` | block-level: paragraphs, lists, nested blockquotes |
| `List` | `ListItem` or `TaskListItem` only |
| `ListItem` / `TaskListItem` | block-level (paragraphs, nested lists) |
| `JsxElement` | block-level (anything MDX allows inside a tag) |
| `JsxFragment` | block-level |

Inline parsers vs block parsers fill `children` differently. Visitor
authors should know which level they are at.

## Mutating

`walk_root` takes `&mut Vec<Node>`. Visitors return `NodeAction`:

```rust
pub enum NodeAction {
    Keep,                   // recurse into children
    KeepSkipChildren,       // do not recurse
    Replace(Vec<Node>),     // splice into parent's children
    Remove,                 // drop from parent's children
}
```

See [`../dmc-transform/visitor.md`](../dmc-transform/visitor.md) for
details.

## Serde

Every `Node` variant + struct derives `Serialize` + `Deserialize`.
Inspect AST as JSON:

```rust
println!("{}", serde_json::to_string_pretty(&doc)?);
```

Or via the `parse` CLI binary (`--json` flag).
