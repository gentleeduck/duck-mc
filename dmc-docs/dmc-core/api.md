# dmc-core API

## `Engine`

```rust
pub struct Engine;

impl Engine {
    pub fn run(
        cfg: &EngineConfig,
        config_path: Option<&Path>,
        diag_engine: &mut DiagnosticEngine<Code>,
    ) -> std::io::Result<()>;
}
```

Path: `dmc::Engine`. Top-level entry. Cleans output (if `cfg.clean`),
warms math cache, processes every collection, writes index.

## `Compiler`

```rust
pub struct Compiler;

impl Compiler {
    pub fn compile(source: &str, diag_engine: &mut DiagnosticEngine<Code>)
        -> CompileOutput;

    pub fn compile_with_pipeline(
        source: &str,
        path: &Path,
        compile_cfg: &CompileConfig,
        diag_engine: &mut DiagnosticEngine<Code>,
    ) -> CompileOutput;
}
```

Path: `dmc::engine::compile::Compiler`. Per-file compile. Math
preprocess, lex, parse, transform pipeline, walker over sinks
(Accumulator + HtmlEmitter + MdxBodyEmitter).

## `CompileConfig`

```rust
pub struct CompileConfig {
    pub markdown_gfm: bool,
    pub emit_html: bool,
    pub emit_body: bool,
    pub mdx_minify: bool,
    pub mdx_output_format: Option<String>,
    pub markdown_remark_plugins: Vec<Value>,
    pub markdown_rehype_plugins: Vec<Value>,
    pub mdx_remark_plugins: Vec<Value>,
    pub mdx_rehype_plugins: Vec<Value>,
    pub copy_linked_files: bool,
    pub output_assets: Option<String>,
    pub output_base: Option<String>,
    pub pretty_code: Option<PrettyCodeOptions>,
    pub math_engine: Option<MathEngine>,
}
```

Path: `dmc::engine::compile::CompileConfig`.

### Methods

```rust
impl CompileConfig {
    pub fn new() -> Self;
    pub fn has_js_plugins(&self) -> bool;
    pub fn for_render(&self) -> Self;
    pub fn pipeline_config(&self, path: &Path) -> PipelineConfig;
    pub fn effective_markdown_remark_plugins(&self) -> Vec<Value>;
    pub fn effective_mdx_remark_plugins(&self) -> Vec<Value>;
    pub fn effective_markdown_rehype_plugins(&self) -> Vec<Value>;
    pub fn effective_mdx_rehype_plugins(&self) -> Vec<Value>;
}
```

`has_js_plugins` returns false when every listed plugin has a native
owner. `effective_*` strip the same plugins from the JSON payload sent
to the sidecar.

## `EngineConfig`

```rust
pub struct EngineConfig {
    pub root: PathBuf,
    pub output_dir: PathBuf,
    pub output_name: Option<String>,
    pub output_format: Option<String>,
    pub clean: bool,
    pub strict: bool,
    pub collections: Vec<Collection>,
    pub include_html: bool,
    pub cache_enabled: bool,
    pub compile: CompileConfig,
}
```

Path: `dmc::engine::config::EngineConfig`. `cache_enabled` defaults to
`true`. `EngineConfig::load(path)` reads `.toml` or `.ts`/`.js`/`.mjs`
configs (TS routes through `bun` or `node + tsx`).

## `Collection`

```rust
pub struct Collection {
    pub name: String,
    pub pattern: String,
    pub base_dir: PathBuf,
    pub schema: Option<Value>,
    pub single: bool,
}

impl Collection {
    pub(crate) fn process(
        &self,
        cfg: &EngineConfig,
        diag_engine: &mut DiagnosticEngine<Code>,
    ) -> Result<CollectionReport, ()>;
}
```

Path: `dmc::engine::collection::Collection`. `process` is `pub(crate)`
and runs only via `Engine::run`.

## `FileCache`

```rust
pub struct FileCache;

impl FileCache {
    pub fn open(dir: PathBuf) -> Option<Self>;
    pub fn key(source: &[u8], path: &Path, cfg_fingerprint: &[u8]) -> String;
    pub fn get(&self, key: &str) -> Option<Value>;
    pub fn put(&self, key: &str, value: &Value);
}

pub fn fingerprint<T: serde::Serialize>(cfg: &T) -> Vec<u8>;
```

Path: `dmc::engine::cache::{FileCache, fingerprint}`. See
[`cache.md`](cache.md) for key composition.

## `Sidecar`

```rust
pub struct Sidecar { /* ... */ }

impl Sidecar {
    pub fn new() -> Option<Self>;
}

pub fn run_sidecar(markdown: &str, cfg: &EngineConfig) -> Option<String>;
```

Path: `dmc::engine::sidecar::{Sidecar, run_sidecar}`. Worker pool
managed via process-global statics.

## `CompileOutput`

```rust
pub struct CompileOutput {
    pub frontmatter: serde_json::Value,
    pub frontmatter_raw: String,
    pub content: String,
    pub html: String,
    pub body: String,
    pub excerpt: String,
    pub metadata: Metadata,
    pub toc: Vec<TocItem>,
    pub imports: Vec<String>,
    pub exports: Vec<String>,
}
```

Path: `dmc::engine::compile::CompileOutput`.

## `Metadata`

```rust
pub struct Metadata {
    pub reading_time: u32,
    pub word_count: u32,
}
```

## `TocItem`

```rust
pub struct TocItem {
    pub title: String,
    pub url: String,
    pub items: Vec<TocItem>,
}
```

## Index emission

```rust
pub mod index {
    pub fn write_index(
        out_dir: &Path,
        collections: &[Collection],
        format: &str,
        config_path: Option<&Path>,
    ) -> std::io::Result<()>;
}
```

Path: `dmc::engine::index::write_index`. Writes `index.js` + `index.d.ts`
re-exporting each `<name>.json`.
