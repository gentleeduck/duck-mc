# dmc-core

Top-level engine. Orchestrates compile + collection + cache + sidecar
+ index emission.

## Engine flow

```mermaid
flowchart TD
    A[EngineConfig::load<br/>toml or ts/js] --> B[Engine::run]
    B --> C{cache_enabled?}
    C -->|yes| D[load math cache from disk]
    C -->|no| E[skip]
    D --> F[for each Collection]
    E --> F
    F --> G[Collection::process]
    G --> H[for each path par_iter]
    H --> I{file cache hit?}
    I -->|yes| J[return cached record]
    I -->|no| K[Compiler::compile_with_pipeline]
    K --> L{has_js_plugins?}
    L -->|yes| M[run_sidecar]
    L -->|no| N[skip sidecar]
    M --> O[wrap mdx + minify]
    N --> O
    O --> P[schema validate]
    P --> Q[write cache]
    Q --> R[collect record]
    J --> R
    R --> S[write {name}.json]
    S --> T[save math cache]
    T --> U[write index.js + index.d.ts]
```

## Crates consumed

| dep | use |
|-----|-----|
| `dmc-lexer` | tokenise source |
| `dmc-parser` | tokens to AST |
| `dmc-transform` | pipeline of AST passes |
| `dmc-codegen` | AST to HTML / MDX body |
| `dmc-schema` | frontmatter validation |
| `dmc-diagnostic` | shared error codes |
| `rayon` | per-file parallelism |
| `globwalk` | collection pattern matching |
| `blake3` | cache keys |

## Files

- [`api.md`](api.md) - public surface
- [`engine.md`](engine.md) - `Engine::run` lifecycle
- [`compile.md`](compile.md) - `Compiler` per-file pipeline
- [`collection.md`](collection.md) - parallel file processing
- [`cache.md`](cache.md) - persistent file + math caches
- [`sidecar.md`](sidecar.md) - JS plugin worker pool
- [`config.md`](config.md) - `EngineConfig` + `CompileConfig` reference
- [`cli.md`](cli.md) - dmc CLI binary
- [`examples.md`](examples.md) - programmatic usage
