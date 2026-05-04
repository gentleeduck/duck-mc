# dmc-parser AST

The single enum `dmc_parser::ast::Node` holds every block, inline, JSX,
and break variant. Source: `dmc-parser/src/ast/node.rs` and
`dmc-parser/src/ast/jsx.rs`.

All structs are `Debug + Clone + PartialEq + Serialize + Deserialize`.

## Node enum

```rust
pub enum Node {
  Document(Document),
  Frontmatter(Frontmatter),
  Import(Import),
  Export(Export),
  Heading(Heading),
  Paragraph(Paragraph),
  Text(Text),
  Bold(Inline),
  Italic(Inline),
  Strikethrough(Inline),
  InlineCode(InlineCode),
  CodeBlock(CodeBlock),
  Link(Link),
  Image(Image),
  HorizontalRule(HorizontalRule),
  Blockquote(Blockquote),
  List(List),
  ListItem(ListItem),
  TaskListItem(TaskListItem),
  Table(Table),
  TableRow(TableRow),
  TableCell(TableCell),
  JsxElement(JsxElement),
  JsxSelfClosing(JsxSelfClosing),
  JsxFragment(JsxFragment),
  JsxExpression(JsxExpression),
  HardBreak(BreakNode),
  SoftBreak(BreakNode),
}
```

`children_of` returns `&[Node]` for any container; leaf variants
return `&[]`. `children_of_mut` returns `Option<&mut Vec<Node>>`.

`TableRow` / `TableCell` look like containers but `TableRow.cells` is
`Vec<TableCell>` (not `Vec<Node>`) so it does not show up in
`children_of`. `TableCell.children` does.

## Variant walk-through

### Document

```rust
pub struct Document {
  pub children: Vec<Node>,
  pub span: Span,
}
```

Root. `children` is the top-level block sequence. `span` is the span
of the first token (or `default_span()` on empty input).

### Frontmatter / Import / Export

```rust
pub struct Frontmatter { pub raw: String, pub span: Span }
pub struct Import      { pub raw: String, pub span: Span }
pub struct Export      { pub raw: String, pub span: Span }
```

`raw` is the verbatim lexeme. Frontmatter YAML is not parsed here;
`dmc-schema` handles validation. `Import` / `Export` raw is the full
`import ... from "..."` / `export ...` line.

### Heading

```rust
pub struct Heading {
  pub level: u8,        // 1..=6
  pub children: Vec<Node>, // inline run
  pub span: Span,
}

impl Heading {
  pub fn slug(&self) -> String;  // slug::slugify of plain text
}
```

`level` comes either from the ATX `#` count or from the setext
underline (`=` -> 1, `-` -> 2). `slug()` walks `children` flattening
text; recurses into `Bold/Italic/Strikethrough/Link/InlineCode`,
skips JSX and images.

### Paragraph

```rust
pub struct Paragraph { pub children: Vec<Node>, pub span: Span }
```

Default block. `children` is an inline-only sequence.

### Text

```rust
pub struct Text { pub value: String, pub span: Span }
```

Backslash escapes are already collapsed by `unescape_markdown` before
the value lands here.

### Inline (wrapper for emphasis)

```rust
pub struct Inline { pub children: Vec<Node>, pub span: Span }
```

Used by `Bold(Inline)`, `Italic(Inline)`, `Strikethrough(Inline)`.
Triple-`***` collapses to `Italic( Bold( ... ) )` per CommonMark,
done in the inline parser.

### InlineCode

```rust
pub struct InlineCode { pub value: String, pub span: Span }
```

`value` is the raw inner text between matching backticks of width
`n` (`CodeStart(n)` ... `CodeEnd(n)`).

### CodeBlock

```rust
pub struct CodeBlock {
  pub lang: Option<String>,
  pub meta: Option<String>,
  pub value: String,
  pub span: Span,
}
```

Shared by fenced and indented forms.

- Fenced: `lang` is the first whitespace-split token of the info
  string; `meta` is the rest (also `None` if empty after trim).
- Indented: both `lang` and `meta` are `None`. Lines have their
  leading 4 spaces stripped and join with `\n`.

### Link

```rust
pub struct Link {
  pub href: String,
  pub title: Option<String>,
  pub children: Vec<Node>,
  pub span: Span,
}
```

`children` is the bracketed inline run. `(href "title")` is parsed by
`split_destination_title` (see [inline-parser.md](inline-parser.md)).
Autolinks (`<https://...>`, `<a@b>`) also produce a `Link` with a
single `Text` child equal to the inner text.

### Image

```rust
pub struct Image {
  pub src: String,
  pub alt: String,           // plain text only
  pub title: Option<String>,
  pub span: Span,
}
```

No nested inline rendering - alt text is flattened during parse.

### HorizontalRule

```rust
pub struct HorizontalRule { pub span: Span }
```

Comes from `TokenKind::ThematicBreak`. (When the token follows a
paragraph and consists of `-` only, the paragraph parser may
re-interpret it as a setext underline; see `parse_paragraph`.)

### Blockquote

```rust
pub struct Blockquote { pub children: Vec<Node>, pub span: Span }
```

Children are block-level (paragraphs, nested blockquotes, etc).
Multi-line `>` runs join into one paragraph; nested depth is built by
the depth-stack walker. See [block-parser.md](block-parser.md).

### List + items

```rust
pub struct List {
  pub ordered: bool,
  pub start: Option<u32>,         // ordered-list starting number
  pub children: Vec<Node>,        // ListItem | TaskListItem
  pub span: Span,
}

pub struct ListItem      { pub children: Vec<Node>, pub span: Span }
pub struct TaskListItem  { pub checked: bool, pub children: Vec<Node>, pub span: Span }
```

For ordered lists `start` is parsed from the first marker (e.g. `3.`
gives `Some(3)`). Items hold inline content directly when tight, and
get wrapped in `Paragraph` when the list is loose (any item has a
paragraph continuation). `TaskListItem` is GFM only and only appears
inside unordered lists.

### Table

```rust
pub enum TableAlign { None, Left, Right, Center }

pub struct Table {
  pub align: Vec<TableAlign>,        // one entry per column
  pub children: Vec<TableRow>,
  pub span: Span,
}

pub struct TableRow {
  pub cells: Vec<TableCell>,
  pub span: Span,
}

pub struct TableCell {
  pub children: Vec<Node>,           // inline-parsed
  pub span: Span,
}
```

`align[i]` is the alignment for column `i` taken from the alignment
row (`|:---|---:|:---:|`). The header is just `children[0]`; there is
no separate `head` / `body` split.

### JSX

```rust
pub struct JsxElement {
  pub name: String,
  pub attrs: Vec<JsxAttr>,
  pub children: Vec<Node>,    // block + inline mix
  pub span: Span,
}

pub struct JsxSelfClosing {
  pub name: String,
  pub attrs: Vec<JsxAttr>,
  pub span: Span,
}

pub struct JsxFragment { pub children: Vec<Node>, pub span: Span }

pub struct JsxExpression {
  pub value: String,         // raw text between `{` and `}`
  pub span: Span,
}
```

```rust
pub struct JsxAttr {
  pub name: String,
  pub value: JsxAttrValue,
  pub span: Span,
}

pub enum JsxAttrValue {
  String(String),       // name="str"
  Expression(String),   // name={expr}  -- raw inner expr text
  Boolean,              // bare name
}
```

`JsxFragment` is what you get when the lexer emits an empty-name open
tag (`<>`). Children inside JSX are parsed via `parse_block` so any
markdown construct (including more JSX) nests freely.

### Breaks

```rust
pub struct BreakNode { pub span: Span }
```

Backs both `Node::HardBreak(BreakNode)` and
`Node::SoftBreak(BreakNode)`. The parser typically consumes break
tokens as paragraph terminators; explicit break nodes appear when an
inline collector exits on a stop-token rather than acting on it.

## Span source

All `span: Span` fields use `duck_diagnostic::Span`. They come from
the cursor token at the start of the construct. `default_span()` in
`ast/mod.rs` is only used as a serde `default` placeholder.
