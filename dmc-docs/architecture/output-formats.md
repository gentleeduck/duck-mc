# Output formats

dmc emits records in two shapes. Pick per collection.

## Formats

| format | body field | render path |
|--------|-----------|-------------|
| `html` (default) | `content`: HTML string | `<div dangerouslySetInnerHTML={__html}>` |
| `mdx` | `body`: JSX module source | `runSync(body)` -> React component |

## Choosing

```ts
collections: {
  posts: {
    name: "Post",
    pattern: "content/posts/**/*.mdx",
    schema: ...,
    output: "html",  // or "mdx"
  },
}
```

Default = `"html"`. Set per-collection.

## When to use html

- Your renderer is plain React `dangerouslySetInnerHTML`.
- You want deterministic, cacheable output.
- You don't need React components inline (only `<a>`, `<table>`,
  etc).
- Fastest runtime: HTML is a string, no JSX evaluation.

Use case: docs site, blog, marketing copy.

## When to use mdx

- Your `.mdx` source uses custom components (`<Callout>`, `<Demo>`,
  `<Tweet>`).
- You want runtime customisation (theme-switch, prop overrides).
- You need React tree access (search, ToC, etc).

Use case: design system docs, interactive tutorials, anything with
embedded JSX components.

## What dmc emits

### html format

```json
{
  "title": "...",
  "slug": "...",
  "content": "<h1>...</h1><p>...</p>"
}
```

`content` is the HTML output of `HtmlEmitter`. JSX components are
emitted as `<div data-component="Callout" data-prop-...></div>` (or
similar) and a runtime layer rehydrates them. For pure markdown, no
rehydration needed.

### mdx format

```json
{
  "title": "...",
  "slug": "...",
  "body": "/* @jsxImportSource react */ const _components = ...; export default function MDXContent(props) { return _jsxs(...); }"
}
```

`body` is a JSX-compiled module source string. The consumer:

```tsx
import * as runtime from "react/jsx-runtime";
import { runSync } from "@mdx-js/mdx";

const { default: MDXContent } = runSync(post.body, runtime);
```

`MDXContent` is a React component. Pass `components={...}` to map
`<Callout>` etc to your implementations.

## Plain markdown body

For raw markdown body (no parsing), use a custom schema field:

```ts
schema: (s) => s.object({
  title: s.string(),
  body: s.markdown(),  // raw markdown body, no parse
})
```

`s.markdown()` is a synthetic field that returns the document body
as-is. dmc skips parse/transform/emit for that field.

Useful for forwarding markdown to a downstream renderer (e.g. an
LLM input pipeline).

## How the choice plumbs through

`Collection::process` reads `output_format`:

```rust
let html = if collection.output_format == "html" {
    Some(html_emit(&compiled.ast))
} else { None };

let body = if collection.output_format == "mdx" {
    Some(mdx_emit(&compiled.ast))
} else { None };

record.insert("content".into(), Value::String(html.unwrap_or_default()));
record.insert("body".into(), Value::String(body.unwrap_or_default()));
```

Both formats can coexist (rare; you'd pay both emit costs).

## Cache key

The cache key includes `output_format` so changing it invalidates.
Switching from `html` to `mdx` on a collection forces a full
recompile of that collection. Use during a migration; switch back
afterwards if needed.

## Performance

| format | emit time per file (1KB doc) |
|--------|----------------------------|
| html | 0.05 ms |
| mdx | 0.15 ms |

mdx is slower because the JSX compilation step (via `@mdx-js`'s
estree-to-source) runs in a JS sidecar. html is pure Rust,
in-process.

## Mixed collections

Different collections can use different formats:

```ts
collections: {
  posts: { output: "html" },
  components: { output: "mdx" },
}
```

dmc handles this by branching per-collection in the engine.

## Future

A third format `react-server` is on the roadmap (RSC-friendly
serialised React tree). Currently in flight; not stable.
