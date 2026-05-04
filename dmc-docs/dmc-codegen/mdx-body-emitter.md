# MdxBodyEmitter

JSX-tree emitter for MDX runtime. Produces a JS function body suitable
for `@mdx-js/react` / `@mdx-js/preact` runtime rendering.

## Signature

```rust
pub struct MdxBodyEmitter { /* private */ }

impl MdxBodyEmitter {
    pub fn new() -> Self;
    pub fn into_parts(self) -> (String, DiagnosticEngine<Code>);
}

impl NodeSink for MdxBodyEmitter {
    fn enter(&mut self, node: &Node, ctx: &WalkCtx);
    fn leave(&mut self, node: &Node, ctx: &WalkCtx);
}

pub fn render_mdx_body(doc: &Document) -> String;
```

Path: `dmc_codegen::MdxBodyEmitter`. Output is a JS string; wrap as
function body or full module per `CompileConfig::mdx_output_format`.

## Output shape

```js
function _createMdxContent(props) {
  const _components = (props && props.components) || {};
  const { Fragment, jsx, jsxs } = arguments[0];
  return jsxs(Fragment, { children: [
    jsxs("h1", { id: "hello", children: [...] }),
    jsxs("p", { children: [...] }),
    // ...
  ]});
}
```

Caller wraps:

```rust
if cfg.mdx_output_format.as_deref() == Some("module") {
    body = wrap_mdx_module(&body, &imports);
}
if cfg.mdx_minify {
    body = minify_js(&body);
}
```

`wrap_mdx_module` adds `import` statements + `export default
_createMdxContent`. `minify_js` runs an swc-style minifier.

## Node mapping

Same node set as `HtmlEmitter`, but expressed as JSX function calls
(`jsx` / `jsxs` / `Fragment`). Inline marks become wrapping
`<strong>` / `<em>` / `<code>` JSX elements. `JsxElement` and
`JsxSelfClosing` pass through directly to support user-authored MDX
components.

## Tables

Tables emit a warning (`GW001 MdxTableUnsupported`) and skip the
node. Reason: emitting a table from raw `Node::Table` to JSX requires
a separate inline-table renderer not yet built. Workaround: run
`DisableGfm` first to convert tables to plain text.

## JsxExpression

`{expr}` markers in MDX become `JsxExpression { value, span }` nodes.
The body emitter inlines the expression verbatim:

```js
{props.foo}
```

The HTML emitter cannot do this and drops them with `GW002
HtmlExpressionDropped`.

## Imports / exports

Top-level `import` / `export` statements in the source are captured
by the parser and stored on `Document`. `wrap_mdx_module` emits them
at the top of the wrapped module:

```js
import Counter from "./counter.tsx";
export const meta = { title: "Hi" };
function _createMdxContent(props) { /* ... */ }
export default _createMdxContent;
```

## Function-body vs module

| `mdx_output_format` | output |
|---------------------|--------|
| `Some("function-body")` | bare function body string; caller wraps |
| `Some("module")` | full module with imports + exports |
| `None` | function-body (default) |

## Pairing with HtmlEmitter

A typical compile runs both emitters on the same Walker pass:

```rust
let mut html_sink = HtmlEmitter::new();
let mut body_sink = MdxBodyEmitter::new();
Walker::new(&doc).walk(&mut [&mut html_sink, &mut body_sink]);
let (html, _) = html_sink.into_parts();
let (body, _) = body_sink.into_parts();
```

Set `CompileConfig::emit_html` / `emit_body` to skip a sink when its
output is unused.
