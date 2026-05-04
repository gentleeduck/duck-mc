# Public API

Every `pub` item exported from `dmc-lexer`. `pub(crate)` and private helpers are not listed.

## `dmc_lexer` (lib.rs)

### `dmc_lexer::Lexer<'eng, 'src>`

```rust
pub struct Lexer<'eng, 'src> {
    pub source: &'src str,
    pub meta: std::sync::Arc<dmc_diagnostic::metadata::SourceMeta>,
    pub tokens: Vec<dmc_lexer::token::Token<'src>>,
    pub start: usize,
    pub current: usize,
    pub line: usize,
    pub column: usize,
    pub diag_engine: &'eng mut duck_diagnostic::DiagnosticEngine<dmc_diagnostic::Code>,
    pub frontmatter_reserved: bool,
}
```

Streaming MDX scanner. `start` marks the begin of the in-progress token. `current` is the scan head. `line`/`column` are zero-based, used for diagnostic spans. `frontmatter_reserved` flips to `true` after one frontmatter block has been emitted, so a later `---` is unambiguously a thematic break.

#### `dmc_lexer::Lexer::new`

```rust
pub fn new(
    source: &'src str,
    meta: std::sync::Arc<dmc_diagnostic::metadata::SourceMeta>,
    diag_engine: &'eng mut duck_diagnostic::DiagnosticEngine<dmc_diagnostic::Code>,
) -> Self
```

Build a fresh lexer. Pre-allocates `tokens` to `source.len() / 8`.

#### `dmc_lexer::Lexer::scan_tokens`

```rust
pub fn scan_tokens(&mut self) -> Result<(), std::io::Error>
```

Scan the entire source into `self.tokens`, then emit `TokenKind::Eof`. Always returns `Ok(())`; errors are reported through the diagnostic engine.

## `dmc_lexer::token` (token.rs)

### `dmc_lexer::token::Token<'src>`

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token<'src> {
    pub kind: dmc_lexer::token::TokenKind,
    pub span: duck_diagnostic::Span,
    pub raw: &'src str,
}
```

One lexed token. `raw` is a borrowed slice of the original source.

#### `dmc_lexer::token::Token::new`

```rust
pub fn new(
    kind: dmc_lexer::token::TokenKind,
    span: duck_diagnostic::Span,
    raw: &'src str,
) -> Self
```

Build a token from kind, span, and raw lexeme.

#### `Display` impl

`Token` implements `core::fmt::Display`, formatting as `Kind("escaped raw")`. Newlines and tabs in `raw` are escaped to `\n` / `\t`.

### `dmc_lexer::token::TokenKind`

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TokenKind { /* ... */ }
```

Tagged enum. See `tokens.md` for the full variant list and what triggers each.

#### `dmc_lexer::token::TokenKind::is_trivia`

```rust
pub fn is_trivia(&self) -> bool
```

Returns `true` for `Whitespace`, `Newline`, `Quote`. Used by `Lexer::emit` to decide what to drop from the stream. See `internals.md` for the actual emit policy (Whitespace is preserved despite being trivia).

#### `Display` impl

`TokenKind` implements `core::fmt::Display`. The string is the variant name without payload (e.g. `Heading(3)` formats as `"Heading"`).

## `dmc_lexer::token` re-exports

The `token` module is `pub mod`, so `dmc_lexer::token::Token` and `dmc_lexer::token::TokenKind` are reachable. The crate does not re-export them at the top level; only `dmc_lexer::Lexer` is reachable directly.

## Modules

- `dmc_lexer::token` (`pub mod`): `Token`, `TokenKind`.
- `dmc_lexer::lexers` (private): per-construct sub-lexers (frontmatter, jsx, code, lists, statements, typography, whitespaces). All `impl` blocks attach `pub(crate)` methods to `Lexer`. None of these methods are public.
- `dmc_lexer::utils` (private): cursor helpers, `peek`, `advance`, byte-level fast scanners. All `pub(crate)`.

Nothing in `lexers/` or `utils.rs` appears in the public surface.
