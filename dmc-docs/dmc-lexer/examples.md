# Examples

End-to-end traces. Each example builds a `dmc_lexer::Lexer`, calls `dmc_lexer::Lexer::scan_tokens`, and shows the resulting `Vec<dmc_lexer::token::Token<'src>>`.

All examples assume this preamble:

```rust
use std::sync::Arc;
use dmc_lexer::Lexer;
use dmc_lexer::token::TokenKind;
use dmc_diagnostic::metadata::{Origin, SourceMeta};
use duck_diagnostic::DiagnosticEngine;

fn meta() -> Arc<SourceMeta> {
    Arc::new(SourceMeta {
        path: Arc::from("doc.mdx"),
        version: 0,
        origin: Origin::Stdin,
    })
}
```

## 1. Plain paragraph

Source:

```text
hello world
```

```rust
let source = "hello world";
let mut engine = DiagnosticEngine::new();
let mut lexer = Lexer::new(source, meta(), &mut engine);
lexer.scan_tokens().unwrap();

// lexer.tokens (kind, raw):
// Text       "hello"
// Whitespace " "
// Text       "world"
// Eof        ""
```

The space is preserved as a `TokenKind::Whitespace` token between the two `Text` tokens.

## 2. Inline code

Source:

```text
run `cargo build` now
```

```rust
let source = "run `cargo build` now";
let mut engine = DiagnosticEngine::new();
let mut lexer = Lexer::new(source, meta(), &mut engine);
lexer.scan_tokens().unwrap();

// lexer.tokens (kind, raw):
// Text          "run"
// Whitespace    " "
// CodeStart(1)  "`"
// Text          "cargo build"
// CodeEnd(1)    "`"
// Whitespace    " "
// Text          "now"
// Eof           ""
```

`TokenKind::CodeStart(u8)` carries the backtick count. The body is a single `Text` because `lex_inline_code` calls `skip_until_any2(b'\n', b'`')` for the closing tick.

## 3. JSX tag with an attribute

Source:

```text
<Button color="red">click</Button>
```

```rust
let source = r#"<Button color="red">click</Button>"#;
let mut engine = DiagnosticEngine::new();
let mut lexer = Lexer::new(source, meta(), &mut engine);
lexer.scan_tokens().unwrap();

// lexer.tokens (kind, raw):
// JsxOpenTagStart   "<"
// JsxTagName        "Button"
// Whitespace        " "
// JsxAttributeName  "color"
// Eq                "="
// String            "red"
// JsxOpenTagEnd     ">"
// Text              "click"
// JsxCloseTagStart  "</"
// JsxTagName        "Button"
// JsxCloseTagEnd    ">"
// Eof               ""
```

The `"` characters that wrap `red` are emitted internally as `TokenKind::Quote` tokens but `emit` drops them as trivia, so the stream surfaces only the inner `String`.

## 4. Bonus: link with hover text

Source:

```text
see [docs](https://x.com) here
```

```rust
let source = "see [docs](https://x.com) here";
let mut engine = DiagnosticEngine::new();
let mut lexer = Lexer::new(source, meta(), &mut engine);
lexer.scan_tokens().unwrap();

// lexer.tokens (kind, raw):
// Text        "see"
// Whitespace  " "
// Bracket     "["
// Text        "docs"
// Bracket     "]"
// ParenOpen   "("
// Text        "https://x.com"
// ParenClose  ")"
// Whitespace  " "
// Text        "here"
// Eof         ""
```

The `Whitespace` after `)` is the bug-fix case described in `internals.md`: previously dropped, now preserved so the inline parser can render the trailing space before `here`.
