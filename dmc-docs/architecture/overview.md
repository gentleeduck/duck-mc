# Architecture overview

dmc compiles MDX to JSON records. Every stage is a separate crate so
each layer can evolve independently and consumers (CLI, napi, tests)
can opt into only what they need.

## Crate graph

```mermaid
flowchart TD
    L[dmc-lexer] --> P[dmc-parser]
    P --> T[dmc-transform]
    T --> CG[dmc-codegen]
    H[dmc-highlight] --> T
    H --> CG
    D[dmc-diagnostic] --> L
    D --> P
    D --> T
    D --> CG
    S[dmc-schema] --> Core
    L --> Core[dmc-core]
    P --> Core
    T --> Core
    CG --> Core
    H --> Core
    Core --> Napi[dmc-napi]
    Core --> CLI[dmc CLI binary]
    Core <--> SC[dmc-sidecar Node helper]
    Napi --> User[user app]
    CLI --> User
```

| crate | layer | reason it is its own crate |
|-------|-------|---------------------------|
| `dmc-diagnostic` | shared | all layers emit through one `Code` enum |
| `dmc-lexer` | tokenise | tokens are stable; multiple parsers could share |
| `dmc-parser` | AST build | grammar surface big enough to warrant isolation |
| `dmc-transform` | AST passes | plugin-style passes; feature-gated |
| `dmc-codegen` | emit | sinks for HTML / MDX body |
| `dmc-highlight` | leaf | breaks codegen <-> transform cycle |
| `dmc-schema` | leaf | Zod-style descriptor compile |
| `dmc-core` | engine | orchestrate + cache + sidecar |
| `dmc-napi` | binding | napi-rs cdylib |
| `dmc-sidecar` | helper | Node JS plugin runner |

## Per-file pipeline

```mermaid
flowchart LR
    Src[source: &str] --> M[Math::preprocess_source]
    M --> Lex[Lexer::scan_tokens]
    Lex --> Par[Parser::parse]
    Par --> Pipe[Pipeline::with_defaults_for cfg]
    Pipe --> Doc[Document mut]
    Doc --> W[Walker]
    W --> Acc[Accumulator]
    W --> Html[HtmlEmitter]
    W --> Body[MdxBodyEmitter]
    Html & Body & Acc --> Out[CompileOutput]
    Out --> Cache[FileCache.put]
    Out --> SC[run_sidecar if has_js_plugins]
    SC --> Final[velite record]
```

## Build pipeline

```mermaid
flowchart TD
    Cfg[EngineConfig::load] --> Run[Engine::run]
    Run --> WarmMath[load math.json cache]
    WarmMath --> Coll[for each Collection]
    Coll --> Glob[globwalk pattern]
    Glob --> Par[par_iter rayon]
    Par --> Hit{cache hit?}
    Hit -->|yes| Rec[record from disk]
    Hit -->|no| Compile[Compiler::compile_with_pipeline]
    Compile --> Side{has_js_plugins?}
    Side -->|yes| Sidecar[run_sidecar]
    Side -->|no| Skip[skip]
    Sidecar --> Schema[schema validate]
    Skip --> Schema
    Schema --> Build[build_velite_record]
    Build --> Put[FileCache.put]
    Put --> Rec
    Rec --> Json[write name.json]
    Json --> Idx[write index.js + index.d.ts]
```

## Data flow

| stage | input | output |
|-------|-------|--------|
| lexer | UTF-8 source | `Vec<Token>` |
| parser | tokens | `Document` (AST root) |
| transform | mutable `Document` | mutated `Document` |
| codegen | `Document` | HTML string + MDX body string + accumulator data |
| schema | parsed frontmatter `Value` | validated `Value` |
| collection | per-file outputs | `<name>.json` |
| index | every collection name | `index.js` + `index.d.ts` |

## Why this layering

- **Parse once, emit many**: one AST feeds Accumulator + HtmlEmitter
  + MdxBodyEmitter through `Walker`. No redundant traversals.
- **Native first, sidecar second**: native transformers absorb every
  popular JS plugin. Sidecar handles the remainder. Gate strips
  duplicate names from the JS payload.
- **Persistent cache**: file output keyed by `(version, source, path,
  cfg)` survives across builds. Math render cache survives too.
- **Feature flags**: every transformer is gated. Slim builds drop
  bundles via `--no-default-features`.
