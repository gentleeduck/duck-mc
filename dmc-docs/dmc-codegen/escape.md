# HTML escape

Two free functions. Both `pub` within the crate, used by `HtmlEmitter`
and `MdxBodyEmitter` for safe text emission.

## `escape_text`

```rust
pub fn escape_text(s: &str) -> String;
```

Path: `dmc_codegen::escape::escape_text`. Use for node text body.

| char | -> |
|------|----|
| `&` | `&amp;` |
| `<` | `&lt;` |
| `>` | `&gt;` |

Does NOT escape `"`. Safe inside an HTML element body.

## `escape_attr`

```rust
pub fn escape_attr(s: &str) -> String;
```

Path: `dmc_codegen::escape::escape_attr`. Use for attribute values.

| char | -> |
|------|----|
| `&` | `&amp;` |
| `"` | `&quot;` |
| `<` | `&lt;` |
| `>` | `&gt;` |

Always wrap attribute values in `"..."` (the emitter does this); the
escape covers everything that would break the wrapping.

## Why two functions

Element body and attribute values have different escape requirements:

- body: `<` / `>` / `&` would break the parser
- attr: same plus `"` (when wrapped in `"..."`)

Splitting keeps each function tight and avoids over-escaping in body
text where `"` is fine.

## Raw HTML escape hatch

`escape_text` is bypassed by the `MathMl` and `MermaidSvg` JSX
self-closing renderers in `HtmlEmitter`. Those write the attribute
value verbatim (after reversing the JSX-attr escape applied during
the source-level math preprocess). See
[`html-emitter.md`](html-emitter.md).

## Usage rules

- Use `escape_text` on `Text` node values, `InlineCode::value`,
  `CodeBlock::value`, link / image text bodies.
- Use `escape_attr` on every attribute value (`href`, `src`, `alt`,
  `title`, `id`, `class`, `data-*`, JSX attr values).
- Never concatenate raw user input directly into the output buffer
  without one of the two.

## Implementation

```rust
pub fn escape_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(c),
        }
    }
    out
}
```

`escape_attr` mirrors with the extra `"` arm. Tight, no allocations
beyond the output buffer.
