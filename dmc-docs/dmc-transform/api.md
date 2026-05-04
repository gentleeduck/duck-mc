# dmc-transform API

Crate root: `dmc_transform`. All exports below are flat re-exports
on the crate root unless otherwise noted.

## Pipeline

### `dmc_transform::Pipeline`

Source: `dmc_transform::pipeline::Pipeline`.

```rust
pub struct Pipeline { /* private */ }
```

Ordered list of `Box<dyn Transformer + Send + Sync>`. Cheap to share
across worker threads. Run order is registration order.

Methods:

- `Pipeline::new() -> Self`
  Empty pipeline. Builder root.

- `Pipeline::add<T: Transformer + Send + Sync + 'static>(self, t: T) -> Self`
  Append `t` to the run order. Returns `self` for chaining.

- `Pipeline::with_defaults() -> Self`
  Equivalent to `with_defaults_for(&PipelineConfig::default())`.

- `Pipeline::with_defaults_for(cfg: &PipelineConfig) -> Self`
  Build the bundled default chain, gated on `cfg` and feature flags.
  See [`pipeline.md`](pipeline.md) for the exact registration order.

- `Pipeline::run(&self, doc: &mut Document, meta: &SourceMeta, engine: &mut DiagnosticEngine<Code>)`
  Apply every registered transformer in order.

- `Pipeline::run_silent(&self, doc: &mut Document)`
  Synth a `Origin::Inline` `SourceMeta`, throwaway `DiagnosticEngine`,
  discard diagnostics. For tests + tooling.

## Transformer

### `dmc_transform::Transformer`

Source: `dmc_transform::pipeline::Transformer`.

```rust
pub trait Transformer {
  fn name(&self) -> &str { "anonymous" }
  fn transform(&self, doc: &mut Document, meta: &SourceMeta, diag_engine: &mut DiagnosticEngine<Code>);
}
```

`&self` so a transformer is shareable. Internal mutable state belongs
behind `Mutex<...>` (see `Mermaid::cache`, `CopyLinkedFiles::map`).

## Visitor / walker

### `dmc_transform::Visitor`

Source: `dmc_transform::visit::Visitor`.

```rust
pub trait Visitor {
  fn visit_node(&mut self, node: &mut Node) -> NodeAction { NodeAction::Keep }
}
```

Default impl recurses everywhere. Override only the variants you need.

### `dmc_transform::NodeAction`

Source: `dmc_transform::visit::NodeAction`.

```rust
pub enum NodeAction {
  Keep,                   // recurse into children
  KeepSkipChildren,       // do not recurse
  Replace(Vec<Node>),     // splice replacements at index, no re-visit
  Remove,                 // drop this node
}
```

`Replace`s are not re-visited: they would loop forever on a transformer
that produces what it just matched. They are descended into on a later
pass when the visitor returns `Keep` for them.

### `dmc_transform::walk_root`

Source: `dmc_transform::visit::walk_root`.

```rust
pub fn walk_root<V: Visitor>(children: &mut Vec<Node>, v: &mut V);
```

Drives the visitor over a `Vec<Node>`, honouring every `NodeAction`.
The standard transformer entrypoint:

```rust
walk_root(&mut doc.children, &mut my_visitor);
```

One helper also lives in `dmc_transform::visit` (not re-exported):

- `walk_children_mut(parent, v)` recurses into a single node's
  inner-children Vec. Tables get special handling: walks into each
  cell's `children` independently.

## Config

### `dmc_transform::PipelineConfig`

Source: `dmc_transform::config::PipelineConfig`.

```rust
#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PipelineConfig {
  pub markdown_gfm: Option<bool>,
  pub pretty_code: Option<PrettyCodeOptions>,
  pub math_engine: Option<MathEngine>,
  pub copy_linked_files: Option<CopyLinkedFilesOptions>,
}
```

All fields optional. The empty config (`PipelineConfig::default()`)
reproduces `Pipeline::with_defaults()`.

Fields:

- `markdown_gfm`: `Some(false)` appends `DisableGfm`.
- `pretty_code`: `Some(_)` -> `PrettyCode::from_options`.
- `math_engine`: `Some(_)` -> `Math::set_engine` (process global).
- `copy_linked_files`: `Some(_)` appends `CopyLinkedFiles`.

A field describing a transformer whose feature is off is silently
ignored, so user settings round-trip across builds.

### `dmc_transform::PrettyCodeOptions`

Source: `dmc_transform::config::PrettyCodeOptions`.

```rust
pub struct PrettyCodeOptions {
  pub theme: PrettyCodeTheme,
  pub default_mode: Option<String>,
}
```

`default_mode` is the mode whose colors fill unprefixed `color` /
`background-color` attrs. Only meaningful for `PrettyCodeTheme::Multi`.
Resolution when unset: `"dark"` if present, else first key.

### `dmc_transform::PrettyCodeTheme`

Source: `dmc_transform::config::PrettyCodeTheme`.

```rust
#[serde(untagged)]
pub enum PrettyCodeTheme {
  Single(String),                  // one bundled theme name
  Multi(BTreeMap<String, String>), // mode -> theme
}
```

JSON: bare string for single, object for multi.
Default: `{ light: "Catppuccin Latte", dark: "Catppuccin Mocha" }`.

### `dmc_transform::MathEngine`

Source: `dmc_transform::config::MathEngine`.

```rust
#[serde(rename_all = "lowercase")]
pub enum MathEngine {
  Katex,   // default; embedded katex via quick-js
  Mathml,  // pulldown-latex -> MathML
}
```

KaTeX matches `rehype-katex` byte-for-byte, slow per expression.
MathML is microseconds, plainer-looking.

### `dmc_transform::CopyLinkedFilesOptions`

Source: `dmc_transform::config::CopyLinkedFilesOptions`.

```rust
pub struct CopyLinkedFilesOptions {
  pub source_dir: PathBuf,
  pub assets_dir: PathBuf,
  pub public_base: String,
}
```

`source_dir` resolves relative `src` / `href`. `assets_dir` receives
the hash-named copies. `public_base` prefixes the rewritten URLs.

## Built-in transformers

All implement `Transformer`. Constructors and field shapes:

| Type                | Path                                           | Constructor                                |
| ------------------- | ---------------------------------------------- | ------------------------------------------ |
| `AutolinkHeadings`  | `dmc_transform::AutolinkHeadings`              | `::new()`, `::default()`                   |
| `BareUrlAutolink`   | `dmc_transform::BareUrlAutolink`               | unit                                       |
| `CodeImport`        | `dmc_transform::CodeImport`                    | `::new()`, `::with_base_dir(p)`            |
| `ComponentPreview`  | `dmc_transform::ComponentPreview`              | `::new(idx, root)`, `::default()`          |
| `ComponentSource`   | `dmc_transform::ComponentSource`               | `::with_base_dir(p)`, `::default()`        |
| `CopyLinkedFiles`   | `dmc_transform::CopyLinkedFiles` (`assets`)    | `::new(src, assets, base)`                 |
| `DisableGfm`        | `dmc_transform::DisableGfm`                    | unit                                       |
| `Emoji`             | `dmc_transform::Emoji` (`emoji`)               | unit                                       |
| `Math`              | `dmc_transform::Math` (`math`)                 | unit; assoc fns below                      |
| `Mermaid`           | `dmc_transform::Mermaid` (`mermaid`)           | `::default()`, `::with_output(p)`          |
| `NpmCommand`        | `dmc_transform::NpmCommand` (`npm-command`)    | unit                                       |
| `PrettyCode`        | `dmc_transform::PrettyCode` (`pretty-code`)    | `::new(name)`, `::from_options(opts)`      |

### Math associated functions

- `Math::preprocess_source(source: &str) -> String`
  Pre-lexer pass. Rewrites raw `$...$` / `$$...$$` to `<MathMl mathml=".."/>`.
  Skips fenced code, inline code, JSX tags. Honours `\$`.

- `Math::render(latex: &str, display: bool) -> String`
  Render to engine-specific HTML. Cached by `(latex, display, engine)`.

- `Math::render_node(latex: &str, display: bool, span: &Span) -> Node`
  Wrap in a `<MathMl mathml=".."/>` `JsxSelfClosing` node.

- `Math::set_engine(engine: MathEngine)`
  Process-wide setter (atomic). Pipeline calls this from
  `cfg.math_engine`.

- `Math::load_cache(path)`, `Math::save_cache(path)`
  JSON persistence. Best effort, errors swallowed.

### CodeImport associated functions

- `CodeImport::with_base_dir(p)` overrides the inferred base dir.
  Without it, the base dir comes from `meta.origin` (`Origin::File`)
  or falls back to cwd with a `Code::BaseDirNotFound` warning.

### Mermaid associated functions

- `Mermaid::with_output(p)`: enable disk cache (`{hash}.svg`).
- `Mermaid::render_cached(&self, source) -> Result<String, String>`:
  cached SVG render. Two-level cache (memory + disk).

### CopyLinkedFiles fields

```rust
pub struct CopyLinkedFiles {
  pub source_dir: PathBuf,
  pub assets_dir: PathBuf,
  pub base_url: String,
  pub name_template: String,        // default "[name]-[hash:8].[ext]"
  pub map: Arc<Mutex<HashMap<String, String>>>,
}
```

`name_template` knobs: `[name]`, `[hash:8]`, `[ext]`.

## Diagnostic codes

Emitted via `dmc_diagnostic::Code` (re-exported from `dmc-diagnostic`):

| Code                          | Source pass         |
| ----------------------------- | ------------------- |
| `BaseDirNotFound`             | code-import, component-source |
| `InvalidLineRange`            | code-import         |
| `ImportFileNotFound`          | code-import         |
| `MissingComponentAttr`        | component-preview, component-source |
| `RegistryIndexUnreadable`     | component-preview   |
| `RegistryIndexMalformed`      | component-preview   |
| `RegistryEntryNotFound`       | component-preview   |
| `RegistrySourceUnreadable`    | component-preview   |
| `ComponentSourceUnreadable`   | component-source    |
| `AssetSourceMissing`          | copy-linked-files   |
| `AssetCopyFailed`             | copy-linked-files   |
| `MmdcUnavailable`             | mermaid             |
| `MermaidRenderFailed`         | mermaid             |
