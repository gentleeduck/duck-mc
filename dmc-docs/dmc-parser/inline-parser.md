# Inline parser

Implementation in `dmc-parser/src/inline.rs`. Driven by
`Parser::collect_inline(stop)` and the helper
`Parser::collect_inline_until_break()`.

## Stop set

```rust
pub(crate) fn collect_inline_until_break(&mut self) -> Vec<Node> {
    self.collect_inline(&|kind| matches!(
        kind,
        TokenKind::HardBreak
        | TokenKind::SoftBreak
        | TokenKind::Eof
        | TokenKind::Heading(_)
        | TokenKind::FrontmatterStart
        | TokenKind::Import
        | TokenKind::Export
        | TokenKind::JsxCloseTagStart
    ))
}
```

`is_top_level_break` is the same set; used by emphasis nesting to
abort if a structural break appears.

## Token mapping

| token | -> Node | notes |
|-------|---------|-------|
| `Text` | `Node::Text` | runs through `unescape_markdown` |
| `Whitespace` | `Node::Text(" ")` | preserves inline spacing around inline elements |
| `Autolink` | `Node::Link` | `<https://...>` or `<a@b.c>` (mailto:) |
| `Bold(n)` | `Node::Bold(Inline)` (or em+strong if n=3) | matched by width |
| `Italic(n)` | `Node::Italic(Inline)` | matched by width |
| `Strike(n)` | `Node::Strikethrough(Inline)` | matched by width |
| `CodeStart(n)` ... `CodeEnd(m)` | `Node::InlineCode` | n must equal m |
| `Bracket` ... `Bracket` ... `ParenOpen` ... `ParenClose` | `Node::Link` | optional `(href "title")` |
| `Bang` ... `Bracket` ... | `Node::Image` | `![alt](src "title")` |
| `JsxOpenTagStart` | recursed via `parse_jsx` | full JSX subtree |
| `ExpressionStart` | `Node::JsxExpression` | `{expr}` |
| `MarkdownCommentStart` | dropped | `{/* ... */}` |

## Triple emphasis

`***x***` lexes as `Bold(3)`. The parser wraps the inner content as
em around strong:

```rust
if open_n == 3 {
    let strong = Node::Bold(Inline { children: inner, span: span.clone() });
    out.push(Node::Italic(Inline { children: vec![strong], span }));
} else {
    out.push(Node::Bold(Inline { children: inner, span }));
}
```

Matches CommonMark `<em><strong>x</strong></em>`.

## Backslash escapes

`unescape_markdown(s)` strips `\` from `\X` pairs in the escapable set:

```
\ * _ ` < > { } [ ] ( ) ! # - $ ~ \
```

So `\*literal\*` renders as `*literal*` (HTML), not `\*literal\*`.

## Link / image title

```rust
fn split_destination_title(body: &str) -> (String, Option<String>) {
    // walk back from end looking for balanced "..." / '...' / (...)
    // require whitespace between dest and the opener
}
```

Body of `(...)` between the link/image parens. Pulls off optional
`"title"` / `'title'` / `(title)` separated by whitespace. Returns
`(href, title)`. Used by both `Link` and `Image` arms.

## Inline-only string

```rust
pub fn parse_inline_str(s: &str) -> Vec<Node>
```

Path: `dmc_parser::parse_inline_str`. Lex + inline-parse a free string.
Used by table cells, which receive raw cell strings rather than
pre-tokenised content. Backticks, bold, italic, links inside cells go
through this.

## Reconstruction limits

The inline parser cannot recover original-source markers from emphasis
wrappers. e.g. `Italic("foo")` does not preserve the underscores
around it. This means math-in-emphasis like `$\sum_{i=1}^{n}$` cannot
be rebuilt from the parsed AST; the source-level math preprocess
(`Math::preprocess_source`) sidesteps this by replacing math regions
with opaque JSX before lexing.

## Bare URLs

`BareUrlAutolink` (a transformer) wraps unprotected `https://...`
runs in Text nodes after parse. The lexer does not detect bare URLs
inline; they arrive as plain Text and the transform does the wrap.
