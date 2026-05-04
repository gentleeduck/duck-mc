# Pipeline

`Pipeline` is the ordered list of `Transformer`s applied to a
`Document`. One uniform place for every feature gate.

## Construction

```rust
pub struct Pipeline { /* private */ }

impl Pipeline {
    pub fn new() -> Self;
    pub fn with_defaults() -> Self;
    pub fn with_defaults_for(cfg: &PipelineConfig) -> Self;
    pub fn add<T: Transformer + Send + Sync + 'static>(self, t: T) -> Self;
    pub fn run(&self, doc: &mut Document, meta: &SourceMeta, engine: &mut DiagnosticEngine<Code>);
    pub fn run_silent(&self, doc: &mut Document);
}
```

Path: `dmc_transform::Pipeline`.

`with_defaults_for(cfg)` is what dmc-core calls. Builds the canonical
chain from the config object. Order is fixed (see below). Add custom
transformers via `.add(...)` if needed.

## Default chain

```rust
let mut p = Pipeline::new()
    .add(CodeImport::new())
    .add(BareUrlAutolink)
    .add(AutolinkHeadings::new());

if cfg.markdown_gfm == Some(false) {
    p = p.add(DisableGfm);
}

#[cfg(feature = "npm-command")]
{ p = p.add(NpmCommand); }

#[cfg(feature = "mermaid")]
{ p = p.add(Mermaid::default()); }

#[cfg(feature = "emoji")]
{ p = p.add(Emoji); }

#[cfg(feature = "math")]
{
    if let Some(engine) = cfg.math_engine {
        Math::set_engine(engine);
    }
    p = p.add(Math);
}

#[cfg(feature = "pretty-code")]
{
    let pc = cfg.pretty_code.as_ref()
        .map(PrettyCode::from_options)
        .unwrap_or_default();
    p = p.add(pc);
}

#[cfg(feature = "assets")]
if let Some(opts) = &cfg.copy_linked_files {
    p = p.add(CopyLinkedFiles::new(/* ... */));
}
```

## Why this order

| step | comes before | reason |
|------|--------------|--------|
| `BareUrlAutolink` | `AutolinkHeadings` | bare URLs inside headings still get wrapped |
| `AutolinkHeadings` | `DisableGfm` | anchor wrap valid even when GFM is off |
| `Mermaid` | `Emoji` / `Math` / `PrettyCode` | mermaid blocks hand off to `mmdc`; later passes must not see them |
| `Math` | `PrettyCode` | math nodes become opaque JSX before highlighting |
| `PrettyCode` | `CopyLinkedFiles` | code blocks rewritten before asset rewrite |

Reorder via custom `Pipeline::new()` if you need different semantics.

## `PipelineConfig`

```rust
pub struct PipelineConfig {
    pub markdown_gfm: Option<bool>,
    pub pretty_code: Option<PrettyCodeOptions>,
    pub math_engine: Option<MathEngine>,
    pub copy_linked_files: Option<CopyLinkedFilesOptions>,
}
```

Path: `dmc_transform::PipelineConfig`. dmc-core builds this from
`CompileConfig` via `compile_cfg.pipeline_config(path)`.

## Run

```rust
pipeline.run(&mut doc, &meta, &mut diag_engine);
```

Each transformer mutates `doc` in place, in registration order. Diag
errors emitted by a transformer accumulate in `diag_engine`; the
caller decides what to do with them.

For tests:

```rust
Pipeline::new()
    .add(MyTransformer)
    .run_silent(&mut doc);
```

Silent variant uses synthetic `Origin::Inline("<test>")` meta and a
throwaway engine.

## Custom

```rust
let pipeline = Pipeline::with_defaults_for(&cfg)
    .add(MyTransformer);
```

Append-only; ordering is fixed for the default chain. To replace the
default chain entirely, use `Pipeline::new()` and add what you need.
