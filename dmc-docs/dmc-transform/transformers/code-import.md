# CodeImport

Inlines source code into code blocks via a `file=` meta directive.
Optional line-range slicing.

## Feature flag

Always on.

## Input

```md
```rust file="../src/lib.rs" {3-12}
```
```

`Node::CodeBlock { lang, meta, value }` where `meta` contains
`file="path"` and optionally `{ranges}`.

## Output

Same `CodeBlock` with `value` replaced by the file contents (sliced
to ranges when present). The original `value` is discarded.

## Path resolution

```rust
fn base_dir(...) -> Option<PathBuf> {
    self.base_dir.clone().or_else(|| match &meta.origin {
        Origin::File(p) => p.parent().map(|p| p.to_path_buf()),
        _ => None,
    })
}
```

Order:

1. Explicit `base_dir` set on the `CodeImport` instance
2. Parent dir of `meta.origin` when `Origin::File`

When neither resolves (Stdin / Inline / Memory origin without base),
emits `TW004 BaseDirNotFound`.

## Range syntax

```
{3-12}      single range, inclusive on both ends
{3,5-7,10}  multiple ranges, comma-separated
```

Lines are 1-based. Out-of-range lines silently clipped.

## Failure modes

| failure | code | severity |
|---------|------|----------|
| missing file | `T001 ImportFileNotFound` | error |
| malformed `{ranges}` | `T002 InvalidLineRange` | error |
| no base dir | `TW004 BaseDirNotFound` | warning |

Per-block failures emit a diagnostic; `value` left as-is.

## API

```rust
pub struct CodeImport {
    pub base_dir: Option<PathBuf>,
}

impl CodeImport {
    pub fn new() -> Self;
    pub fn with_base(p: impl Into<PathBuf>) -> Self;
}
```

Path: `dmc_transform::CodeImport`. Default uses meta-derived base
dir (typical case).

## Example

Source:

````md
```rust file="../src/lib.rs" {1-3}
```
````

If `../src/lib.rs` is:

```rust
fn main() {
    println!("hi");
}

fn other() {}
```

After CodeImport pass:

````md
```rust file="../src/lib.rs" {1-3}
fn main() {
    println!("hi");
}
```
````

(`meta` retained so downstream tooling can see the import; `value`
populated.)

## Composing

`CodeImport` runs early in the pipeline (before `PrettyCode`), so
imported code goes through the highlighter just like inline code.
