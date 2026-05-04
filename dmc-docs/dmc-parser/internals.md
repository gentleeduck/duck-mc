# Parser internals

State, helpers, dispatch. For when the public API + per-area docs
are not enough.

## State

```rust
pub struct Parser<'eng, 'tokens> {
    pub tokens: Vec<Token<'tokens>>,
    pub pos: usize,
    pub meta: Arc<SourceMeta>,
    pub diag_engine: &'eng mut DiagnosticEngine<Code>,
}
```

| field | use |
|-------|-----|
| `tokens` | full token stream from the lexer (consumed by reference) |
| `pos` | cursor index into `tokens` |
| `meta` | shared `SourceMeta` for span construction |
| `diag_engine` | mutable borrow of the shared diagnostic sink |

Both lifetimes:

- `'eng` = engine borrow; lives only for the parse call
- `'tokens` = token raw `&str` slices; tied to the original source

## Cursor helpers

```rust
fn peek(&self) -> Option<&Token<'tokens>>;
fn peek_kind(&self) -> Option<&TokenKind>;
fn peek_raw(&self) -> Option<&'tokens str>;
fn current_span(&self) -> Span;
fn advance(&mut self);
```

All `pub(crate)`. The pattern is `peek_kind()` + match arms; advance
on commit.

## Block dispatch

```rust
pub(crate) fn parse_block(&mut self) -> Option<Node>;
```

Top-down. See `block-parser.md` for the cases. `None` means the
cursor advanced but emitted no node (e.g. stray break, comment).

## Inline dispatch

```rust
pub(crate) fn collect_inline_until_break(&mut self) -> Vec<Node>;
pub(crate) fn collect_inline(&mut self, stop: &dyn Fn(&TokenKind) -> bool) -> Vec<Node>;
```

`collect_inline_until_break` calls `collect_inline` with the stop set
listed in `inline-parser.md`. Custom stops (e.g. emphasis matching)
build their own predicates.

## Mutating tokens

`parse_list` (ordered) mutates `self.tokens[pos].raw` to trim a
leading `.` from the first text token after a marker. This is the
only place the parser mutates the lexer's raw slice. Keep in mind
when tracing.

## Speculative parsing

`try_parse_table` saves `self.pos`, runs forward, restores on
mismatch. Same idea applies to ordered-list-marker validation
(`raw.parse::<u32>()` failures).

## Diagnostic emission

```rust
self.diag_engine.emit(Diagnostic::new(Code::TableShapeMismatch, "row count mismatch"));
```

Most parser diagnostics are errors (`P***` codes). Warnings
(`PW***`) are emitted when the parser recovers but the source was
ambiguous (`RecoveredUnterminatedJsx`, `HeadingLevelClamped`).

## `parse_inline_str`

```rust
pub fn parse_inline_str(s: &str) -> Vec<Node>;
```

Spawns a fresh lexer + parser for an inline string. Used by table
cells and any consumer that needs to inline-parse without a full
document. Returns the `Vec<Node>` of inline children.

## Public surface

```rust
pub use parser::{Parser, parse, parse_inline_str};
```

`parse(source: &str) -> Document` is the convenience entry; production
callers use `Parser::new(tokens, meta, diag_engine).parse()`.

## Thread safety

Parser holds a mutable borrow of the diag engine. Not `Send` while
borrowed. To parse in parallel, give each thread its own
`DiagnosticEngine` and merge after.

## Token lifetime

`Token::raw` is `&'src str` slicing into the original source string.
The parser holds the `Vec<Token>` by value but the inner `&str`
references all point into the same source buffer. Source must outlive
the parse pass; consumers usually keep both alive together (compile
returns owned strings).

## Error recovery vs panic

Parser never panics. Every malformed construct produces a diagnostic
+ the parser keeps going. Even unterminated JSX synthesises a
self-close so the document is still emittable. Callers can check
`diag_engine` for severity counts and decide whether to fail.

## Performance

Roughly 50-100 us per 1 KB of source. Most of the cost lives in the
inline parser (emphasis matching does small linear scans). Block
dispatch is a flat match. Tokens are already in memory; no I/O.
