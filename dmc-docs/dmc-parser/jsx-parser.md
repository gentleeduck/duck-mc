# JSX parser

JSX subtree parsing lives in `dmc-parser/src/jsx.rs`. Cursor enters
on `TokenKind::JsxOpenTagStart`, exits past the matching close (or
self-close).

## Entry

```rust
pub(crate) fn parse_jsx(&mut self) -> Node;
```

Returns one of:

| AST node | when |
|----------|------|
| `JsxElement` | named tag with children: `<Card>...</Card>` |
| `JsxSelfClosing` | named tag with `/>`: `<Img src="..."/>` |
| `JsxFragment` | nameless: `<>...</>` |

## Tag name

After `JsxOpenTagStart`, optionally a `JsxTagName` token. Empty name
becomes a fragment.

```rust
let name = if let Some(t) = self.peek() {
    if matches!(t.kind, TokenKind::JsxTagName) {
        let n = t.raw.to_string();
        self.advance();
        n
    } else {
        String::new()
    }
} else {
    String::new()
};
```

## Whitespace

`skip_jsx_ws()` swallows `Whitespace` tokens at every JSX boundary:

```rust
fn skip_jsx_ws(&mut self) {
    while matches!(self.peek_kind(), Some(TokenKind::Whitespace)) {
        self.advance();
    }
}
```

Called after the open `<`, after the tag name, between attrs, and
inside the close tag. Required because the lexer now preserves
inline whitespace tokens (needed for inline spacing around links);
JSX tag-internal whitespace is structural noise.

## Attribute parsing

```rust
fn parse_jsx_attrs(&mut self) -> Vec<JsxAttr>;
```

Loops while `peek_kind() == JsxAttributeName`. For each attribute:

1. Advance past the name; record it.
2. `skip_jsx_ws()`.
3. If `Eq` follows:
   - `skip_jsx_ws()`
   - If `String` -> `JsxAttrValue::String(raw)`
   - If `ExpressionStart` -> walk to `ExpressionEnd`, capture inner as `JsxAttrValue::Expression(text)`
   - Otherwise -> `JsxAttrValue::Boolean`
4. Else -> `JsxAttrValue::Boolean` (bare attribute name)
5. Push `JsxAttr`, `skip_jsx_ws()`.

## Self-close vs body

After attributes:

| token | meaning |
|-------|---------|
| `JsxSelfClosingEnd` (`/>`) | emit `JsxSelfClosing`, return |
| `JsxOpenTagEnd` (`>`) | open body; recurse into children |
| anything else | warn `RecoveredUnterminatedJsx`, synthesise self-close |

## Body recursion

After `JsxOpenTagEnd`, the parser loops:

```rust
loop {
    match self.peek_kind() {
        Some(TokenKind::JsxCloseTagStart) => {
            self.advance();
            self.skip_jsx_ws();
            if matches!(self.peek_kind(), Some(TokenKind::JsxTagName)) {
                self.advance();
            }
            self.skip_jsx_ws();
            if matches!(self.peek_kind(), Some(TokenKind::JsxCloseTagEnd)) {
                self.advance();
            }
            break;
        },
        Some(TokenKind::Eof) | None => break,
        _ => {
            let before = self.pos;
            if let Some(node) = self.parse_block() {
                children.push(node);
            }
            if self.pos == before {
                self.advance();
            }
        },
    }
}
```

Block-level inside JSX. Each child is parsed via `parse_block`, the
same dispatcher used at the document root. Lets MDX freely nest:

```mdx
<Card>
  ## Heading
  some paragraph
  <Button>click</Button>
</Card>
```

## Expression marker

```rust
pub(crate) fn parse_jsx_expression(&mut self) -> Node;
```

Cursor on `ExpressionStart`. Walks to `ExpressionEnd` capturing raw
inner text. Returns `Node::JsxExpression { value }`.

`HtmlEmitter` drops `JsxExpression` with `GW002 HtmlExpressionDropped`
(HTML cannot run JS). `MdxBodyEmitter` inlines the expression
verbatim.

## Markdown comment

```rust
pub(crate) fn skip_md_comment(&mut self);
```

Consumes a `{/* ... */}` block. Comment content is discarded;
returns no node.

## Errors

| condition | code | severity |
|-----------|------|----------|
| missing close after attrs | `PW004 RecoveredUnterminatedJsx` | warning, parser synthesises self-close |
| EOF inside body | break and continue (no node added past this point) |
| mismatched close-tag name | `P010 MismatchedJsxCloseTag` | error (close is still consumed) |
