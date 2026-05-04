# HtmlEmitter

Static HTML emitter. One sink among the three driven by `Walker`.
Produces a single `String` for SSR / SSG.

## Signature

```rust
pub struct HtmlEmitter { /* private */ }

impl HtmlEmitter {
    pub fn new() -> Self;
    pub fn into_parts(self) -> (String, DiagnosticEngine<Code>);
}

impl NodeSink for HtmlEmitter {
    fn enter(&mut self, node: &Node, ctx: &WalkCtx);
    fn leave(&mut self, node: &Node, ctx: &WalkCtx);
}

pub fn render_html(doc: &Document) -> String;
```

Path: `dmc_codegen::HtmlEmitter`. Convenience: `dmc_codegen::render_html`
spawns its own walker + sink and returns just the string.

## Node mapping

| Node variant | HTML |
|--------------|------|
| `Document` | nothing (children inline) |
| `Frontmatter` | nothing (consumed by Accumulator) |
| `Heading(h)` | `<h{level} id="{slug}">` ... `</h{level}>` (autolink wrap may add `<a>`) |
| `Paragraph` | `<p>` ... `</p>` |
| `Text(t)` | escape_text(t.value) |
| `Bold(i)` | `<strong>` ... `</strong>` |
| `Italic(i)` | `<em>` ... `</em>` |
| `Strikethrough(i)` | `<del>` ... `</del>` |
| `InlineCode(c)` | `<code>{escape_text(c.value)}</code>` |
| `CodeBlock(cb)` | `<pre><code class="gentledmc-language-{lang}">{escape_text(cb.value)}</code></pre>` (only when no PrettyCode pass ran) |
| `Link(l)` | `<a href="{href}" title="{title}?">` ... `</a>` |
| `Image(i)` | `<img src alt title?>` |
| `HorizontalRule` | `<hr/>` |
| `Blockquote` | `<blockquote>` ... `</blockquote>` |
| `List(l)` | `<ul>` or `<ol start="..">` |
| `ListItem` | `<li>` ... `</li>` |
| `TaskListItem(t)` | `<li class="task-list-item"><input type="checkbox" disabled checked?>` ... `</li>` |
| `Table(t)` | `<table>` ... `</table>` with thead / tbody |
| `TableRow` / `TableCell` | `<tr>` / `<td>` |
| `JsxElement(e)` | `<{name}>` ... `</{name}>` (raw HTML escape hatch for `MathMl` / `MermaidSvg`) |
| `JsxSelfClosing(s)` | self-close emit (escape hatch for raw HTML) |
| `JsxFragment` | nothing (children inline) |
| `JsxExpression(e)` | dropped (with `GW002 HtmlExpressionDropped` warning) |
| `HardBreak` / `SoftBreak` | newline / space |

## Raw HTML escape hatch

Two element names are recognised as raw-HTML pasters:

```rust
"MermaidSvg" => {
    if let Some(attr) = s.attrs.iter().find(|a| a.name == "svg")
        && let JsxAttrValue::String(svg) = &attr.value
    {
        self.out.push_str(svg);  // verbatim
    }
}
"MathMl" => {
    if let Some(attr) = s.attrs.iter().find(|a| a.name == "mathml")
        && let JsxAttrValue::String(mathml) = &attr.value
    {
        let unescaped = mathml.replace("&quot;", "\"").replace("&amp;", "&");
        self.out.push_str(&unescaped);
    }
}
```

Used by `dmc_transform::Mermaid` (SVG output of `mmdc`) and
`dmc_transform::Math` (KaTeX / MathML output of the math engine). The
`Math` paster reverses the JSX-attr escape applied by
`Math::preprocess_source`.

## Heading slug + autolink

When `dmc_transform::AutolinkHeadings` ran, every `Heading` already
contains a `Link` child wrapping the inline content. The emitter
respects that wrap directly; no extra logic. The `id="..."` on the
heading comes from `Heading::slug()`.

## Diagnostics

`HtmlEmitter` emits two warnings:

- `GW001 MdxTableUnsupported` (from `MdxBodyEmitter`, not html)
- `GW002 HtmlExpressionDropped` when a `JsxExpression` node is
  encountered (HTML cannot run JS; tell the user to use the MDX body
  emitter for full JSX support).

## Escapes

Use `escape::escape_text` (text content) or `escape::escape_attr`
(attribute values). The functions are private; consumers do not call
them directly.

| char | -> |
|------|---|
| `&` | `&amp;` |
| `<` | `&lt;` |
| `>` | `&gt;` |
| `"` (in attrs only) | `&quot;` |
