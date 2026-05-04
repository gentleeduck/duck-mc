# ComponentSource

Inlines a file's contents as a code block. Sibling of `ComponentPreview`
that takes a `path=` instead of `name=`.

## Feature flag

Always on.

## Input

```mdx
<ComponentSource path="../src/button.tsx" />
```

`Node::JsxSelfClosing` or `Node::JsxElement` with `path="..."` attr.

## Behaviour

1. Resolve `path` relative to the source file (or explicit `base_dir`).
2. Read the file.
3. Replace the JSX node with a code block.

## Failure modes

| failure | code | severity |
|---------|------|----------|
| unreadable file | `T007 ComponentSourceUnreadable` | error |
| missing `path` attr | `TW002 MissingComponentAttr` | warning |
| no base dir | `TW004 BaseDirNotFound` | warning |

## API

```rust
pub struct ComponentSource {
    pub base_dir: Option<PathBuf>,
}
```

Path: `dmc_transform::ComponentSource`. Same base-dir resolution as
`CodeImport`.

## Example

```mdx
<ComponentSource path="./button.tsx" />
```

After pass:

````md
```tsx
// contents of ./button.tsx
```
````

## Difference vs CodeImport

| | ComponentSource | CodeImport |
|-|-----------------|------------|
| input | `<ComponentSource path="..."/>` JSX | code block with `file=` meta |
| line range | no | yes (`{1-3}`) |
| use case | embed component code in prose | embed file in pre-existing code block |
