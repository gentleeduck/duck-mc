# dmc-parser API

Every public item with its canonical path. `pub(crate)` helpers
listed only when they shape the public flow.

## `src/lib.rs`

Module re-exports.

```rust
pub mod ast;          // dmc_parser::ast
pub mod parser;       // dmc_parser::parser

pub use parser::{Parser, parse, parse_inline_str};
```

`block`, `inline`, `jsx`, `table` are private impl modules attaching
methods to `Parser`.

## `src/parser.rs`

### `dmc_parser::Parser`

```rust
pub struct Parser<'eng, 'tokens> {
  pub tokens: Vec<Token<'tokens>>,
  pub meta: Arc<SourceMeta>,
  pub pos: usize,
  pub diag_engine: &'eng mut DiagnosticEngine<Code>,
}
```

- `'tokens` ties borrowed lexemes to the source buffer.
- `'eng` ties the engine borrow to the caller.

### `dmc_parser::Parser::new`

```rust
pub fn new(
  tokens: Vec<Token<'tokens>>,
  meta: Arc<SourceMeta>,
  diag_engine: &'eng mut DiagnosticEngine<Code>,
) -> Self
```

Constructs a parser at `pos = 0`.

### `dmc_parser::Parser::parse`

```rust
pub fn parse(&mut self) -> Document
```

Drives the top-level loop until EOF. Force-advances on no-progress so a
malformed token cannot wedge.

### `dmc_parser::parse`

```rust
pub fn parse(source: &str) -> Document
```

Lex + parse in one shot. Diagnostics are dropped. Use for tests + the
`parse` bin only; production callers build their own
`DiagnosticEngine`.

### `dmc_parser::parse_inline_str`

```rust
pub fn parse_inline_str(s: &str) -> Vec<crate::ast::Node>
```

Lex `s`, run the inline collector, return inline nodes. Used by
`table::make_row` to inline-parse cell strings.

### Internal helpers (pub(crate))

Not exported but documented to read the source:

- `peek`, `peek_kind`, `peek_raw`, `advance`, `is_eof`, `current_span`.
- `emit_diagnostic`, `diag`, `warn` (severity sugar over `diag`).

## `src/block.rs`

All methods are `impl Parser`. Internally `pub(crate)` only.

- `parse_block(&mut self) -> Option<Node>` - top dispatcher.
- `parse_frontmatter` - `Frontmatter` from `FrontmatterStart .. End`.
- `parse_list(ordered, indent) -> Node` - `List` with nested sub-list
  handling via `indent`.
- `parse_one_list_item(ordered) -> Node` - `ListItem` or
  `TaskListItem` (GFM `[ ]` / `[x]`).
- `parse_blockquote -> Node` - depth-stack walker; merges multi-line
  into one `Paragraph` until a `HardBreak`.
- `parse_indented_code -> Node` - 4-space indented code block.
- `parse_paragraph -> Node` - default fallback; also detects setext
  underline and rewrites as `Heading`.
- `parse_heading -> Node` - ATX heading.
- `parse_code_block -> Node` - fenced code; splits info string at
  first whitespace into `(lang, meta)`.
- `import_node`, `export_node` - raw lexeme wrappers.

Helpers:

- `peek_leading_indent`, `append_to_item`, `ensure_loose_item` -
  list-item plumbing.
- `count_line_blockquote_markers`, `consume_blockquote_markers`,
  `close_blockquote_level` - blockquote stack ops.
- `setext_underline_level`, `eat_setext_underline` - setext detection.

## `src/inline.rs`

`impl Parser`:

- `collect_inline_until_break -> Vec<Node>` - stops at any break or
  block boundary token.
- `collect_inline_for_list_item -> Vec<Node>` - same but eats one
  leading whitespace.
- `collect_inline(stop) -> Vec<Node>` - generic collector; takes a
  `&dyn Fn(&TokenKind) -> bool` stop predicate.
- `is_top_level_break(k) -> bool` - shared predicate for breaks +
  block boundaries.

Static helpers:

- `split_destination_title(body) -> (String, Option<String>)` - parse
  link/image `(href "title")` body. Walks back from end for a
  balanced quoted/`(...)` title.
- `unescape_markdown(s) -> String` - collapse `\X` for the
  CommonMark escapable set: `` \\ \* \_ \` \< \> \{ \} \[ \] \( \) \! \# \- \$ \~ ``.

UTF-8 helper: `utf8_char_len(b) -> usize` (file-local, not on
`Parser`).

## `src/jsx.rs`

`impl Parser`:

- `parse_jsx -> Node` - returns `JsxElement`, `JsxSelfClosing`, or
  `JsxFragment` (when name empty). Emits
  `Code::RecoveredUnterminatedJsx` if the open tag never closes and
  synthesises a self-close.
- `parse_jsx_expression -> Node` - standalone `{expr}`.
- `skip_md_comment` - eats `{/* ... */}`.

File-local:

- `skip_jsx_ws` - drops `Whitespace` tokens between JSX bits.
- `parse_jsx_attrs -> Vec<JsxAttr>` - name / `name="str"` /
  `name={expr}` / bare boolean.

## `src/table.rs`

`impl Parser`:

- `try_parse_table -> Option<Node>` - speculative GFM table parse.
  Rolls back `pos` on mismatch so the caller can fall through to
  `parse_paragraph`.

File-local helpers:

- `collect_line_text` - rebuild the upcoming line into a string and
  token count, stopping at any break or block boundary.
- `looks_like_table_row(s) -> bool` - line trimmed must start + end
  with `|` and have `>= 2` pipes.
- `parse_alignment_row(s) -> Option<Vec<TableAlign>>` - parses
  `|:---|---:|:---:|`.
- `split_cells(s) -> Vec<String>` - strip outer `|` then split.
- `make_row(cells, span) -> TableRow` - re-lex each cell via
  `parse_inline_str` so inline markdown inside cells works.

## `src/ast/mod.rs`

```rust
pub mod jsx;
pub mod node;

pub use jsx::*;
pub use node::*;

pub fn default_span() -> duck_diagnostic::Span;
```

`default_span` is the serde `default` for fields whose `Span` is not
`Serialize` / `Deserialize`.

## `src/ast/node.rs`

### `dmc_parser::ast::Node`

28-variant enum. Full walk-through in [ast.md](ast.md). Has two
helpers:

```rust
impl Node {
  pub fn children_of(node: &Node) -> &[Node];
  pub fn children_of_mut(node: &mut Node) -> Option<&mut Vec<Node>>;
}
```

Both return children for variants that have a `children: Vec<Node>`
field; `&[]` / `None` for leaves.

### Block structs

- `dmc_parser::ast::Document { children, span }`
- `dmc_parser::ast::Frontmatter { raw, span }`
- `dmc_parser::ast::Import { raw, span }`
- `dmc_parser::ast::Export { raw, span }`
- `dmc_parser::ast::Heading { level: u8, children, span }` plus
  `impl Heading { pub fn slug(&self) -> String }` - URL-anchor slug
  built lazily from plain-text contents via `slug::slugify`.
- `dmc_parser::ast::Paragraph { children, span }`
- `dmc_parser::ast::CodeBlock { lang, meta, value, span }`
- `dmc_parser::ast::Blockquote { children, span }`
- `dmc_parser::ast::List { ordered: bool, start: Option<u32>, children, span }`
- `dmc_parser::ast::ListItem { children, span }`
- `dmc_parser::ast::TaskListItem { checked: bool, children, span }`
- `dmc_parser::ast::HorizontalRule { span }`
- `dmc_parser::ast::Table { align: Vec<TableAlign>, children: Vec<TableRow>, span }`
- `dmc_parser::ast::TableRow { cells: Vec<TableCell>, span }`
- `dmc_parser::ast::TableCell { children, span }`

### Inline structs

- `dmc_parser::ast::Text { value: String, span }`
- `dmc_parser::ast::Inline { children, span }` (used by Bold / Italic
  / Strikethrough)
- `dmc_parser::ast::InlineCode { value, span }`
- `dmc_parser::ast::Link { href, title: Option<String>, children, span }`
- `dmc_parser::ast::Image { src, alt, title: Option<String>, span }`
- `dmc_parser::ast::BreakNode { span }` (used by HardBreak / SoftBreak)

### JSX structs

- `dmc_parser::ast::JsxElement { name, attrs: Vec<JsxAttr>, children, span }`
- `dmc_parser::ast::JsxSelfClosing { name, attrs, span }`
- `dmc_parser::ast::JsxFragment { children, span }`
- `dmc_parser::ast::JsxExpression { value: String, span }`

### Enums

```rust
pub enum TableAlign { None, Left, Right, Center }
```

## `src/ast/jsx.rs`

```rust
pub struct JsxAttr {
  pub name: String,
  pub value: JsxAttrValue,
  pub span: Span,
}

pub enum JsxAttrValue {
  String(String),
  Expression(String),
  Boolean,
}
```

`Boolean` is the bare-name case (`<Foo disabled />`).
