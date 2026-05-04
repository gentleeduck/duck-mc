# Extension points

Where to plug in custom behaviour without forking dmc.

## Native transformer

Most powerful, fastest. Implement `Transformer` in Rust, register in
`Pipeline`. See [`../dmc-transform/writing-a-transformer.md`](../dmc-transform/writing-a-transformer.md).

| use when |
|----------|
| You ship the workspace + can rebuild the napi binary |
| Performance matters |
| You need access to the AST |

## Foreign unified plugin (sidecar)

Drop a `remark-*` / `rehype-*` plugin into config; runs in dmc-sidecar
Node child.

```ts
defineConfig({
  markdown: {
    remarkPlugins: [myRemarkPlugin],
    rehypePlugins: [[myRehypePlugin, { option: true }]],
  },
});
```

| use when |
|----------|
| Plugin already exists for the unified ecosystem |
| You do not control the workspace (npm-only consumer) |
| Per-file IPC overhead is acceptable |

## Custom loader

Read non-MDX files into the engine. See [`../dmc-napi/loaders.md`](../dmc-napi/loaders.md).

```ts
defineLoader({
  test: /\.yaml$/,
  load({ path, value }) { return { data: parse(value) }; },
});
```

| use when |
|----------|
| Source file is not MDX (YAML, TOML, CSV, custom) |
| You need to pre-process before schema validation |

## Schema transform / refine

Mutate or validate during schema parsing.

```ts
schema: s
  .object({ title: s.string() })
  .refine(d => d.title.length > 0, "title required")
  .transform(d => ({ ...d, title: d.title.toUpperCase() })),
```

| use when |
|----------|
| Validation rule beyond built-in `min` / `max` / `regex` |
| Computed fields (slug, permalink, derived metadata) |

## Hooks: `prepare` / `complete`

Mutate the entire collection set.

```ts
defineConfig({
  prepare(data) {
    data.posts = (data.posts as Post[]).filter(p => !p.draft);
  },
  complete(data) {
    writeFileSync("rss.xml", buildRss(data.posts));
  },
});
```

| use when |
|----------|
| Cross-collection logic (sort, filter, derive) |
| Side effects (write extra files, notifications) |

## Per-collection callback

```ts
defineCollection({
  name: "post",
  pattern: "posts/**/*.mdx",
  schema: ...,
  onRecord(rec) {
    return { ...rec, permalink: `/blog/${rec.slug}` };
  },
});
```

| use when |
|----------|
| Per-record post-processing |
| Computed fields specific to one collection |

## Node sink

Add a sink alongside `HtmlEmitter` / `MdxBodyEmitter`. Implement
`NodeSink`. Used internally by `Accumulator`; not exposed via napi
yet.

| use when |
|----------|
| You want to walk the AST during the same DFS as the emitters |
| Avoid double-walking for downstream tasks (e.g. backlink graph) |

## Codegen escape hatch

Need to emit raw HTML from a transformer? Use the `MathMl` /
`MermaidSvg` JsxSelfClosing pattern:

```rust
Node::JsxSelfClosing(JsxSelfClosing {
    name: "MyRawHtml".into(),
    attrs: vec![JsxAttr {
        name: "html".into(),
        value: JsxAttrValue::String(raw_html_string),
        span: span.clone(),
    }],
    span,
})
```

Then add a recognisable arm in `dmc-codegen/src/html.rs::jsx_self_closing`:

```rust
"MyRawHtml" => {
    if let Some(attr) = s.attrs.iter().find(|a| a.name == "html")
        && let JsxAttrValue::String(html) = &attr.value
    {
        self.out.push_str(html);
    }
}
```

| use when |
|----------|
| Pre-rendered HTML needs to skip text-escaping |
| Same pattern as Mermaid SVG / KaTeX HTML |

## Cargo feature flag

Gate optional code paths.

```toml
[features]
my-feature = ["dep:my-crate"]
```

```rust
#[cfg(feature = "my-feature")]
mod my_pass;
```

| use when |
|----------|
| Adding a heavy dep some consumers do not want |
| Optional transformer (mermaid, math, etc) |

## Diagnostic code

For new error / warning types in your transformer:

1. Add a variant to `dmc-diagnostic/src/lib.rs` `Code` enum.
2. Add the canonical id to `Code::code()`.
3. Add severity to `Code::severity()`.
4. Doc under [`../dmc-diagnostic/codes.md`](../dmc-diagnostic/codes.md).

## What is NOT extensible (yet)

| feature | status |
|---------|--------|
| Lexer plugins | not extensible; modify the Cargo workspace |
| Parser grammar plugins | not extensible; modify the Cargo workspace |
| Custom node variants in AST | not extensible; reuse `JsxElement` with custom `name` |
| Pluggable cache backend | only file-system; could be added |
| Pluggable index emit | only ESM / CJS; could be added |

Open via PR if any of these block you.
