# BareUrlAutolink

Wraps unprotected `https://...` and `http://...` URLs in `Node::Text`
with a `Node::Link`. Mirrors GFM autolink behaviour for bare URLs.

## Feature flag

Always on.

## Input

Any `Node::Text { value }` whose `value` contains `http://` or
`https://`. The transformer walks `Paragraph`, `Bold`, `Italic`,
`Strikethrough`, `ListItem`, `TableCell`, etc, looking for `Text`
children inside.

## Behaviour

For each occurrence:

1. Find the URL boundary (whitespace, `)`, `<`, `>`, end-of-string).
2. Split the text into `before` + `url` + `after`.
3. Replace the original `Text` with `Text(before) + Link(url) + Text(after)`.

Repeats for multiple URLs in one text node.

## Output

Source:

```md
See https://example.com for info
```

After pass, the paragraph children are:

```
Text("See ")
Link(href="https://example.com", children: [Text("https://example.com")])
Text(" for info")
```

After render:

```html
<p>See <a href="https://example.com">https://example.com</a> for info</p>
```

## Skipped contexts

URLs inside the following are NOT wrapped (they are not `Text` nodes):

- `Node::InlineCode`
- `Node::CodeBlock`
- `Node::JsxElement` / `Node::JsxSelfClosing` / `Node::JsxExpression`
- existing `Node::Link` / `Node::Image`
- attribute values

So `` `see https://x.com` `` stays as inline code, and
`[](https://x.com)` is not double-wrapped.

## API

```rust
pub struct BareUrlAutolink;

impl Transformer for BareUrlAutolink {
    fn name(&self) -> &str { "bare-url-autolink" }
}
```

Path: `dmc_transform::BareUrlAutolink`.

## Order

Runs before `AutolinkHeadings` so bare URLs inside heading text are
wrapped before the heading anchor wrap.

## Boundary chars

The URL extends from `http(s)://` until any of:

- whitespace (space, tab, newline)
- `)`
- `<` / `>`
- end-of-string

Trailing `.`, `,`, `;`, `!`, `?` are kept as part of the URL (matches
GFM behaviour). Authors who want trailing punctuation outside the link
should add a space.

## Example

Source:

```md
visit https://x.com, then go to https://y.com.
```

Renders:

```html
<p>visit <a href="https://x.com,">https://x.com,</a> then go to <a href="https://y.com.">https://y.com.</a></p>
```

(Trailing punctuation in URLs is the GFM-compliant choice.)

## Why a transformer

The dmc lexer focuses on tokens and structural markers. Wrapping
runs of text into links is a post-parse responsibility; doing it
inline at lex time would over-tokenise.
