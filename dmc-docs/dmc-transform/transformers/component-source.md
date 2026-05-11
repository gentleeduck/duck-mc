# `component-source`

Resolves `<ComponentSource name="..." />` JSX nodes, attaching the raw
source file content as an attribute so the consumer's
`<ComponentSource>` runtime renders the example without filesystem
access at render time.

- **Source:** `dmc-transform/src/builtin/component_source.rs`
- **Feature flag:** none
- **Config:** registry path (consumer-driven)

## Input

```mdx
<ComponentSource name="accordion" />
```

## Output

Same JSX node with resolved source inlined as a `__rawString__` (or
similar) attribute that the consumer renders through
[`pretty-code`](./pretty-code.md) or its own `<pre>` override.

## Difference vs `code-import`

- [`code-import`](./code-import.md) replaces the JSX with a fenced code
  block - meant for plain code embedding.
- `component-source` keeps the JSX wrapper intact - meant for rich
  component-aware previews (Copy button, file tree, multi-file
  switcher).
