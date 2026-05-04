# ComponentPreview

Looks up a component by name in a registry index JSON file and inlines
its source as a code block.

## Feature flag

Always on.

## Input

JSX elements like:

```mdx
<ComponentPreview name="button" />
```

`Node::JsxSelfClosing` or `Node::JsxElement` with `name="..."` attr.

## Behaviour

1. Read `registry_index` JSON (path configured on the transformer).
2. Find entry by `name`.
3. Read the entry's first source file.
4. Replace the JSX node with a code block containing the file contents.

## Failure modes

| failure | code | severity |
|---------|------|----------|
| index unreadable | `T003 RegistryIndexUnreadable` | error |
| index not JSON | `T004 RegistryIndexMalformed` | error |
| name not in index | `T005 RegistryEntryNotFound` | error |
| source file unreadable | `T006 RegistrySourceUnreadable` | error |
| missing `name` attr | `TW002 MissingComponentAttr` | warning |

## API

```rust
pub struct ComponentPreview {
    pub registry_index: Option<PathBuf>,
}

impl ComponentPreview {
    pub fn new() -> Self;
    pub fn with_index(p: impl Into<PathBuf>) -> Self;
}
```

Path: `dmc_transform::ComponentPreview`.

## Example registry index

```json
{
  "button": {
    "files": ["registry/button/button.tsx"]
  }
}
```

Source MDX:

```mdx
<ComponentPreview name="button" />
```

After pass:

````md
```tsx
// contents of registry/button/button.tsx
```
````

## Use case

Doc sites that display the same component code in multiple pages can
keep one source-of-truth file and reference it by name. Edits to the
component show up everywhere on next build.
