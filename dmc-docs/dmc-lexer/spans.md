# Span construction

Every token carries a `Span` pointing back at the source byte range.
Used for diagnostics and incremental work.

## Type

```rust
pub struct Span {
    pub start: usize,   // byte offset, inclusive
    pub end: usize,     // byte offset, exclusive
}
```

Path: `dmc_lexer::Span`. Re-exported by `dmc_diagnostic`.

Bytes, not chars. UTF-8 source means `end - start` does not equal
char count for non-ASCII. The lexer never splits inside a multi-byte
UTF-8 sequence, so spans always point at char boundaries.

## How spans are built

The lexer keeps `pos: usize` (current byte offset). Each `emit_token`
captures `start` from before the consume and `end` from after:

```rust
let start = self.pos;
self.consume_while(|c| c.is_alphanumeric());
let end = self.pos;
self.emit(TokenKind::Word, Span { start, end });
```

Whitespace tokens get spans the same way; the lexer no longer
filters whitespace as trivia (preserving it preserves the
inter-token byte boundaries needed for accurate inline parsing).

## Multi-line tokens

Code blocks, blockquotes, JSX expressions can span many lines. The
span is one contiguous range; the lexer tracks newlines internally
to drive line/column conversion later.

## Line + column resolution

`dmc_diagnostic::resolve(span, source)` converts to (line, column):

```rust
pub fn resolve(span: Span, source: &str) -> (Position, Position) {
    let mut line = 1;
    let mut col = 1;
    let mut start = None;
    let mut end = None;
    for (i, c) in source.char_indices() {
        if start.is_none() && i >= span.start { start = Some((line, col)); }
        if end.is_none() && i >= span.end { end = Some((line, col)); break; }
        if c == '\n' { line += 1; col = 1; } else { col += 1; }
    }
    (start.unwrap_or((line, col)), end.unwrap_or((line, col)))
}
```

O(span.end) walk. Cheap for diagnostic rendering; never on the hot
path.

## Span propagation through parse

The parser attaches the span of the consumed token range to each
AST node:

```rust
pub struct Heading {
    pub depth: u8,
    pub children: Vec<Node>,
    pub span: Span,
}
```

`span.start` = first token's start; `span.end` = last token's end.
Nested nodes have spans inside their parent's span.

## Span propagation through transform

Transformers preserve spans on rewritten nodes. When a transformer
synthesises a new node (e.g. `BareUrlAutolink` wrapping a URL), the
new node copies the source `Text`'s span for its parts.

## Span propagation through emit

`HtmlEmitter` does not emit spans by default. With
`include_spans = true` (debug builds), it adds `data-span="42-58"`
attributes to elements. Useful for source-mapping HTML back to MDX
in a browser DevTools scenario.

## Diagnostics

Diagnostics carry one or more `Label`s, each pinned to a `Span`:

```rust
let diag = Diagnostic::new(Code::E001, "invalid character")
    .with_label(Label::primary(span, Some("here".into())));
```

The renderer (in `dmc-diagnostic`) calls `resolve` once per span to
pretty-print:

```
file.mdx:12:3
  |
12 | $$broken
  |   ^^^^^^ here
```

## Edge cases

- Empty source: spans 0..0.
- Trailing whitespace before EOF: lexer emits a final `EOF` with
  zero-length span at the source length.
- Synthesised tokens (parser recovery): use `Span { start: pos, end: pos }`
  where `pos` is the recovery point. Diagnostics referencing them
  show a column-only marker, no width.

## Why bytes

UTF-8 byte offsets are O(1) to slice from a `&str`. Char offsets
require O(n) walk. The lexer slices source dozens of times per file
(token text, code block bodies); bytes keep that cheap.

## Span arithmetic

```rust
impl Span {
    pub fn merge(self, other: Span) -> Span {
        Span { start: self.start.min(other.start), end: self.end.max(other.end) }
    }
}
```

Used to build a parent span from children. The parser composes
heading spans this way:

```rust
let span = first_token.span.merge(last_token.span);
```
