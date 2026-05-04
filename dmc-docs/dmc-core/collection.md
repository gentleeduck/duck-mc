# Collection

One `Collection` = one glob + one schema + one output JSON file.

## Definition

```rust
pub struct Collection {
    pub name: String,        // "doc", "post", etc
    pub pattern: String,     // glob like "docs/**/*.mdx"
    pub base_dir: PathBuf,
    pub schema: Option<Value>,
    pub single: bool,        // emit one record (not array)
}
```

Path: `dmc::engine::collection::Collection`.

## Process flow

```mermaid
flowchart TD
    A[glob walk base_dir/pattern] --> B[paths Vec]
    B --> C[par_iter]
    C --> D[read source]
    D --> E{cache hit?}
    E -->|yes| Z[record from cache]
    E -->|no| F[compile_with_pipeline]
    F --> G{has_js_plugins}
    G -->|yes| H[run_sidecar]
    G -->|no| I[skip]
    H --> J[mdx wrap + minify]
    I --> J
    J --> K[schema validate]
    K --> L[build_velite_record]
    L --> M[cache.put]
    M --> Z
    Z --> N[collect outcomes]
    N --> O[merge diag engines]
    O --> P[write {name}.json]
```

## Parallelism

```rust
let outcomes: Vec<(Option<Value>, DiagnosticEngine<Code>)> = paths
    .par_iter()
    .map(|path| { ... })
    .collect();
```

Rayon `par_iter` runs each file in its own thread. Each gets a
private `DiagnosticEngine`; merged into the caller's after the
collect (avoids `RefCell` / lock contention on every emit).

## Schema validation

```rust
let collection_schema = self.schema.as_ref().and_then(|d| {
    dmc_schema::compile_descriptor(d).ok()
});

let validated_frontmatter = match (&collection_schema, &compiled.frontmatter) {
    (Some(schema), fm) if !fm.is_null() => {
        let ctx = build_schema_ctx(path, &cfg.root, &compiled, cfg);
        schema.parse(fm, &ctx).unwrap_or_else(|e| {
            local_diag_engine.emit(diag!(...));
            compiled.frontmatter.clone()
        })
    },
    _ => compiled.frontmatter.clone(),
};
```

Schema descriptor (JSON object from `s.object(...)` on the JS side) gets
compiled to a runtime validator once per collection. Parse failures
emit a diagnostic; record falls back to raw frontmatter.

## Output

```rust
let out_path = cfg.output_dir.join(format!("{}.json", self.name));
let json = if self.single {
    serde_json::to_string_pretty(&records[0])?
} else {
    serde_json::to_string_pretty(&records)?
};
std::fs::write(&out_path, json)?;
```

Single-record collections (e.g. site config) emit one object. Plural
collections emit an array. Filename is `<name>.json` in
`cfg.output_dir`.

## `CollectionReport`

```rust
pub struct CollectionReport {
    pub name: String,
    pub records: usize,
    pub output_path: PathBuf,
}
```

Returned to `Engine::run` for index emission.
