# dmc-lexer internals

## Dispatch

`Lexer::scan_tokens` loops:

```rust
while !self.is_eof() {
    self.start = self.current;
    let c = self.advance();
    self.lex_tokens(c);
}
self.emit(TokenKind::Eof);
```

`lex_tokens(c)` is a single big match on the just-consumed char. Each
arm calls a sub-lexer:

```rust
match c {
    '\n' => self.lex_newline(),
    '\r' | '\t' | ' ' => self.lex_whitespace(),
    '(' => self.emit(TokenKind::ParenOpen),
    ')' => self.emit(TokenKind::ParenClose),
    '[' => self.lex_link(),
    '!' if self.peek() == Some('[') => self.lex_image(),
    '`' => self.lex_code(),
    // ...
    _ => self.lex_text(),
}
```

Sub-lexers live in `src/lexers/*.rs` (typography.rs, whitespaces.rs,
lists.rs, ...). Each consumes more chars until its own boundary,
then calls `self.emit(kind)`.

## emit() rule

```rust
fn emit(&mut self, kind: TokenKind) {
    let length = self.current - self.start;
    if kind.is_trivia() && !matches!(kind, TokenKind::Whitespace) {
        self.start = self.current;
        return;  // drop Newline / Quote
    }
    let span = Span::from_zero_based(
        self.meta.path.clone(),
        self.line, self.column, length,
    );
    self.tokens.push(Token::new(kind, span, self.current_lexeme()));
    self.start = self.current;
}
```

`Whitespace` was previously dropped as trivia; now preserved. Reason:
text-after-link patterns like `[x](url) y` lost the space, producing
`xy`. The inline parser has a `Whitespace` arm that emits `Text(" ")`.
JSX paths skip whitespace via `skip_jsx_ws`.

## Span

```rust
Span::from_zero_based(path, line, column, length)
```

`line` and `column` are 1-based in the lexer's own state (incremented
on `\n` and per advance). `from_zero_based` converts at construction.
`length` = byte count of the lexeme.

## Backslash escapes

`lex_text` swallows pairs in the standard escapable set:

```rust
if c == '\\' {
    if let Some(nx) = self.peek_next()
        && matches!(nx, '\\' | '*' | '_' | '`' | '<' | '>' | '{' | '}' | '[' | ']' | '(' | ')' | '!' | '#' | '-')
    {
        self.advance(); // backslash
        self.advance(); // escaped char
        continue;
    }
}
```

Both bytes go into the resulting Text token raw. The inline parser's
`unescape_markdown` strips the backslash from the rendered value
later.

## JSX detection

```rust
'<' if self.peek() == Some('!') => self.lex_comment(),
'<' if self.is_angle_autolink() => self.lex_angle_autolink(),
'<' if matches!(self.peek_next(), Some(c) if c.is_ascii_alphabetic() || c == '/' || c == '>') => self.lex_jsx_tag(),
'<' => self.lex_text(),
```

Order matters. Comment first (`<!--`), then angle-autolink
(`<https://...>`), then JSX, then plain `<` text.

## Indented code marker

```rust
let line_leading = matches!(kind, TokenKind::Whitespace)
    && self.column == length
    && length >= 4
    && self.current_lexeme().chars().all(|c| c == ' ');
```

(Removed by the `Whitespace` preservation change; the parser now
disambiguates indented-code-block from nested-list-marker via
`parse_block`'s lookahead. See `dmc-docs/dmc-parser/block-parser.md`.)

## Re-run cost

The lexer is byte-bound. ~1 GB/sec on warm cache. Negligible compared
to parse + transform time on real docs.
