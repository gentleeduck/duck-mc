# dmc-schema API

## `compile_descriptor`

```rust
pub fn compile_descriptor(d: &Value) -> Result<Schema, String>;
```

Path: `dmc_schema::compile_descriptor`. Compiles a JSON descriptor
(emitted by the JS `s.object(...)` builder) into a runtime validator.

## `Schema`

```rust
pub struct Schema { /* private */ }

impl Schema {
    pub fn parse(&self, value: &Value, ctx: &Ctx) -> Result<Value, String>;
}
```

Path: `dmc_schema::Schema`. `parse` returns the validated (and
possibly transformed) value or a String error.

## `Ctx`

```rust
pub struct Ctx {
    pub path: PathBuf,           // file path of the record
    pub root: PathBuf,           // collection content root
    pub meta: ContextMeta,       // reading time, word count, slug, etc
}
```

Path: `dmc_schema::Ctx`. Carries per-record context to schema
transforms (e.g. computed `slug` field).

## Descriptor JSON shape

The JS-side `s.object(...)` serialises to a tagged JSON tree:

```json
{
  "kind": "object",
  "fields": {
    "title": { "kind": "string", "max": 99 },
    "tags":  { "kind": "array", "item": { "kind": "string" }, "optional": true },
    "date":  { "kind": "date" }
  }
}
```

`compile_descriptor` walks the tree and builds the runtime validator.
Any unknown `kind` returns `Err` from `compile_descriptor`.

See [`descriptors.md`](descriptors.md) for the full descriptor catalogue.
