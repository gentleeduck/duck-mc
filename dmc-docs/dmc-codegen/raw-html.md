# Raw HTML emission

Two cases where dmc emits HTML verbatim instead of escaping:

1. `Node::Html` (raw `<div>` blocks in MDX)
2. `Node::JsxSelfClosing` for the `MathMl` and `MermaidSvg` escape
   hatches

## Node::Html

Produced by the parser when a markdown block looks like raw HTML
(starts with `<` followed by a known tag or `!--` comment). The
parser stores the raw text; `HtmlEmitter` writes it as-is.

```rust
Node::Html(html) => {
    out.push_str(&html.value);
}
```

No escape. No validation. Trusts the source.

This is intentional: MDX users who write raw HTML expect it to pass
through. Sanitisation, if needed, is the consumer's responsibility
(rehype-sanitize plugin via the sidecar, or a downstream HTML
sanitiser before render).

## MathMl

```mdx
<MathMl mathml="<math><mn>1</mn></math>" />
```

The `Math` transformer emits this self-closing JSX during the
preprocess pass. `HtmlEmitter` recognises the component name:

```rust
if name == "MathMl" {
    if let Some(attr) = jsx.attrs.iter().find(|a| a.name == "mathml") {
        let mathml = unescape_jsx_attr(&attr.value);
        out.push_str(&mathml);
    }
    return;
}
```

`unescape_jsx_attr` reverses the `&quot;` and `&amp;` substitutions
applied during preprocess. The result lands in the output as raw
MathML (which is HTML-compatible and renders natively in modern
browsers).

## MermaidSvg

Same shape:

```mdx
<MermaidSvg svg="<svg>...</svg>" />
```

Emitted by the `Mermaid` transformer after `mermaid-cli` returns an
SVG. `HtmlEmitter` pastes the SVG verbatim. `unescape_jsx_attr`
reverses the JSX attribute escape.

## Why JSX attributes for raw HTML

The escape hatch had three constraints:

1. **Round-trip through the AST**: the transformer outputs
   structured nodes, not strings. The parser sees the JSX
   element naturally.
2. **No new AST variant**: `JsxSelfClosing` already exists. Adding
   `RawHtmlInjection` would mean teaching every layer about it.
3. **Escapability**: the LaTeX/SVG content has `<`, `>`, `"`. A JSX
   attribute value must escape those. The reverse step happens
   only at emit time.

## unescape_jsx_attr

```rust
fn unescape_jsx_attr(s: &str) -> String {
    s.replace("&quot;", "\"").replace("&amp;", "&")
}
```

Order matters: replace `&quot;` first (otherwise `&amp;quot;` would
double-decode). Only those two entities are introduced by the
preprocess; no need to handle `&lt;` etc.

## In MdxBodyEmitter

When emitting the JSX-compiled module body (output_format = `mdx`),
`MathMl` and `MermaidSvg` are emitted as-is in the JSX tree. The
React/MDX runtime renders them as components, which the consumer
implements:

```tsx
function MathMl({ mathml }: { mathml: string }) {
  return <div dangerouslySetInnerHTML={{ __html: mathml }} />;
}

function MermaidSvg({ svg }: { svg: string }) {
  return <div dangerouslySetInnerHTML={{ __html: svg }} />;
}
```

The MDX path keeps the strings escaped in the source; rendering
de-escapes via `dangerouslySetInnerHTML`.

## Sanitisation note

Anything raw-emitted (Html, MathMl content, MermaidSvg content) is
trusted source. If your content is user-submitted, run a sanitiser
downstream:

- HTML output: rehype-sanitize via sidecar, or a server-side
  sanitiser before sending to the browser.
- MDX output: not sanitisable post-compile (it's executable JSX);
  sanitise the input MDX before compilation.
