# `component-preview`

Resolves `<ComponentPreview name="..." />` JSX nodes against a registry of
preview components, attaching source + metadata attributes so the
consumer's `<ComponentPreview>` runtime can render the example without
re-reading the registry at render time.

- **Source:** `dmc-transform/src/builtin/component_preview.rs`
- **Feature flag:** none
- **Config:** registry path / mapping (consumer-driven)

## Input

```mdx
<ComponentPreview
  name="accordion-1"
  description="A simple accordion."
/>
```

## Output

The original JSX node is preserved; the transformer attaches resolved
attributes:

```mdx
<ComponentPreview
  name="accordion-1"
  description="A simple accordion."
  source="<file content>"
  files={[...]}
/>
```

The exact attr set depends on what the registry exposes for that name.

## Use case

Powers component galleries / docs sites where every example lives in a
real registry directory and the doc just references it by name.
