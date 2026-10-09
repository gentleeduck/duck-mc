#![deny(clippy::all)]

use napi::bindgen_prelude::*;
use napi_derive::napi;
use serde_json::Value;
use std::path::{Path, PathBuf};

use dmc::Engine;
use dmc::engine::collection::Collection as CollectionDef;
use dmc::engine::compile::{CompileConfig, Compiler};
use dmc::engine::config::EngineConfig;
use dmc_diagnostic::Code;
use duck_diagnostic::DiagnosticEngine;

/// Compile-time options, the subset of `ContentOptions` that changes how one
/// source is compiled. `build` takes the same settings through `BuildInput`;
/// this is for callers compiling a string they already hold.
#[napi(object)]
pub struct CompileOptions {
  pub markdown_gfm: Option<bool>,
  pub mdx_minify: Option<bool>,
  pub mdx_output_format: Option<String>,
  pub copy_linked_files: Option<bool>,
  /// Syntax highlighting. Without it code fences compile to plain `<pre>`,
  /// which is why a consumer would otherwise have to highlight in the browser.
  pub pretty_code: Option<Value>,
  pub mermaid: Option<Value>,
  pub allow_dangerous_html: Option<bool>,
}

pub fn compile_config_from(opts: Option<CompileOptions>) -> Result<CompileConfig> {
  let Some(o) = opts else { return Ok(CompileConfig::new()) };
  Ok(CompileConfig {
    markdown_gfm: o.markdown_gfm.unwrap_or(true),
    emit_html: true,
    emit_body: true,
    mdx_minify: o.mdx_minify.unwrap_or(false),
    mdx_output_format: o.mdx_output_format,
    copy_linked_files: o.copy_linked_files.unwrap_or(false),
    pretty_code: pretty_code_of(&o.pretty_code)?,
    mermaid: mermaid_of(&o.mermaid)?,
    allow_dangerous_html: o.allow_dangerous_html.unwrap_or(false),
    ..CompileConfig::new()
  })
}

#[napi]
pub fn compile(source: String, options: Option<CompileOptions>) -> Result<Value> {
  let cfg = compile_config_from(options)?;
  let mut diag = DiagnosticEngine::<Code>::new();
  let out = Compiler::compile_with_pipeline(&source, Path::new("."), &cfg, &mut diag);
  serde_json::to_value(&out).map_err(|e| Error::from_reason(e.to_string()))
}

/// Render a LaTeX fragment to KaTeX HTML. Output is byte-compatible with
/// `rehype-katex`; pair with `katex.min.css` for glyph rendering.
#[napi]
pub fn latex_to_html(latex: String, display: bool) -> Result<String> {
  let opts = katex::Opts::builder()
    .display_mode(display)
    .output_type(katex::OutputType::Html)
    .build()
    .map_err(|e| Error::from_reason(e.to_string()))?;
  katex::render_with_opts(&latex, &opts).map_err(|e| Error::from_reason(e.to_string()))
}

#[napi]
pub fn compile_many(sources: Vec<String>, options: Option<CompileOptions>) -> Result<Vec<Value>> {
  // One config for the batch: the syntax bundle behind `prettyCode` is parsed once
  // per process, so a shared config is what makes a batch cheaper than N calls.
  let cfg = compile_config_from(options)?;
  let mut diag = DiagnosticEngine::<Code>::new();
  sources
    .into_iter()
    .map(|s| {
      let out = Compiler::compile_with_pipeline(&s, Path::new("."), &cfg, &mut diag);
      serde_json::to_value(&out).map_err(|e| Error::from_reason(e.to_string()))
    })
    .collect()
}

#[napi(object)]
pub struct CollectionInput {
  pub name: String,
  pub pattern: String,
  pub base_dir: String,
  pub schema: Option<Value>,
  pub single: Option<bool>,
  /// Component name -> schema descriptor; validated in the body and emitted onto the record.
  pub components: Option<Value>,
}

#[napi(object)]
pub struct BuildInput {
  pub output_dir: String,
  pub collections: Vec<CollectionInput>,
  pub root: Option<String>,
  pub strict: Option<bool>,
  pub clean: Option<bool>,
  pub output_assets: Option<String>,
  pub output_base: Option<String>,
  pub output_name: Option<String>,
  pub output_format: Option<String>,
  pub markdown_remark_plugins: Option<Value>,
  pub markdown_rehype_plugins: Option<Value>,
  pub mdx_remark_plugins: Option<Value>,
  pub mdx_rehype_plugins: Option<Value>,
  pub copy_linked_files: Option<bool>,
  pub mdx_output_format: Option<String>,
  pub mdx_minify: Option<bool>,
  pub markdown_gfm: Option<bool>,
  pub include_html: Option<bool>,
  pub cache_enabled: Option<bool>,
  /// Route every plugin through the sidecar; drop every native transformer.
  pub force_sidecar: Option<bool>,
  /// Per-plugin sidecar preference. Each listed name routes through the
  /// sidecar and drops its matching native transformer. Recognised names:
  /// `remark-gfm`, `remark-math`, `remark-emoji`, `rehype-pretty-code`,
  /// `shiki`, `rehype-katex`, `rehype-mathjax`, `rehype-slug`,
  /// `rehype-autolink-headings`, `mermaid`, `rehype-mermaid`, `remark-mermaid`.
  pub prefer_sidecar: Option<Vec<String>>,
  /// Free-form JSON, deserialised into `MermaidOptions`.
  pub mermaid: Option<Value>,
  /// Free-form JSON, deserialised into `PrettyCodeOptions`.
  pub pretty_code: Option<Value>,
  /// SEC-010: opt in to raw embedded HTML passthrough (CommonMark
  /// "unsafe" mode). Defaults to `false` — attacker-supplied `<script>`
  /// markup is escaped/dropped rather than emitted verbatim. Enable only
  /// when the markdown source is fully trusted.
  pub allow_dangerous_html: Option<bool>,
}

#[napi(object)]
pub struct BuildCollectionReport {
  pub name: String,
  pub output_path: String,
  pub records: u32,
}

#[napi(object)]
pub struct DiagnosticReport {
  /// Stable error code, e.g. `T007`, `TW005`, `E001`.
  pub code: String,
  /// One of `bug | error | warning | help | note`.
  pub severity: String,
  /// Human-readable summary line.
  pub message: String,
  /// Optional follow-up help text (e.g. `bundled themes: ...`).
  pub help: Option<String>,
  /// First label's source-file path; enables `path:line:col` formatting JS-side.
  pub file: Option<String>,
  /// First label's 1-based line.
  pub line: Option<u32>,
  /// First label's 1-based column.
  pub column: Option<u32>,
}

#[napi(object)]
pub struct BuildReport {
  pub diagnostics: Vec<DiagnosticReport>,
  pub collections: Vec<BuildCollectionReport>,
  pub errors: Vec<String>,
}

fn array_or_default(v: Option<Value>) -> Vec<Value> {
  match v {
    Some(Value::Array(a)) => a,
    _ => Vec::new(),
  }
}

fn pretty_code_of(v: &Option<Value>) -> Result<Option<dmc::PrettyCodeOptions>> {
  v.as_ref()
    .map(|v| {
      serde_json::from_value::<dmc::PrettyCodeOptions>(v.clone())
        .map_err(|e| Error::from_reason(format!("invalid prettyCode config: {e}")))
    })
    .transpose()
}

fn mermaid_of(v: &Option<Value>) -> Result<Option<dmc::MermaidOptions>> {
  v.as_ref()
    .map(|v| {
      serde_json::from_value::<dmc::MermaidOptions>(v.clone())
        .map_err(|e| Error::from_reason(format!("invalid mermaid config: {e}")))
    })
    .transpose()
}

#[napi]
pub fn build(input: BuildInput) -> Result<BuildReport> {
  let compile = CompileConfig {
    markdown_gfm: input.markdown_gfm.unwrap_or(true),
    emit_html: true,
    emit_body: true,
    mdx_minify: input.mdx_minify.unwrap_or(false),
    mdx_output_format: input.mdx_output_format,
    markdown_remark_plugins: array_or_default(input.markdown_remark_plugins),
    markdown_rehype_plugins: array_or_default(input.markdown_rehype_plugins),
    mdx_remark_plugins: array_or_default(input.mdx_remark_plugins),
    mdx_rehype_plugins: array_or_default(input.mdx_rehype_plugins),
    copy_linked_files: input.copy_linked_files.unwrap_or(false),
    output_assets: input.output_assets,
    output_base: input.output_base,
    pretty_code: pretty_code_of(&input.pretty_code)?,
    mermaid: mermaid_of(&input.mermaid)?,
    math_engine: None,
    force_sidecar: input.force_sidecar.unwrap_or(false),
    prefer_sidecar: input.prefer_sidecar.unwrap_or_default(),
    allow_dangerous_html: input.allow_dangerous_html.unwrap_or(false),
  };

  let cfg = EngineConfig {
    output_dir: PathBuf::from(input.output_dir),
    root: PathBuf::from(input.root.unwrap_or_else(|| ".".into())),
    strict: input.strict.unwrap_or(false),
    clean: input.clean.unwrap_or(false),
    output_name: input.output_name,
    output_format: input.output_format,
    include_html: input.include_html.unwrap_or(false),
    cache_enabled: input.cache_enabled.unwrap_or(true),
    collections: input
      .collections
      .into_iter()
      .map(|c| CollectionDef {
        name: c.name,
        pattern: c.pattern,
        base_dir: PathBuf::from(c.base_dir),
        schema: c.schema,
        single: c.single.unwrap_or(false),
        components: c.components,
      })
      .collect(),
    compile,
  };

  let mut diag = DiagnosticEngine::<Code>::new();
  // napi-rs only carries a `String` across the FFI boundary; `help` /
  // labels survive only via `BuildReport.diagnostics`. This message-only
  // conversion is the abort path; structured detail still ships back.
  if let Err(d) = Engine::run(&cfg, None, &mut diag) {
    use duck_diagnostic::DiagnosticCode;
    return Err(Error::from_reason(format!("{}: {}", d.code.code(), d.message)));
  }

  // Engine writes `<output_dir>/<name>.json` per collection; re-read to
  // get the record count so the JS wrapper can do its unified post-pass.
  let collections: Vec<BuildCollectionReport> = cfg
    .collections
    .iter()
    .map(|c| {
      let output_path = cfg.output_dir.join(format!("{}.json", c.name));
      let records = std::fs::read_to_string(&output_path)
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| match v {
          serde_json::Value::Array(a) => Some(a.len() as u32),
          serde_json::Value::Object(_) => Some(1),
          _ => None,
        })
        .unwrap_or(0);
      BuildCollectionReport { name: c.name.clone(), output_path: output_path.to_string_lossy().into_owned(), records }
    })
    .collect();

  let diagnostics: Vec<DiagnosticReport> = diag
    .iter()
    .map(|d| {
      use duck_diagnostic::DiagnosticCode;
      let first_label = d.labels.first();
      DiagnosticReport {
        code: d.code.code().to_string(),
        severity: severity_label(d.severity),
        message: d.message.clone(),
        help: d.help.clone(),
        file: first_label.map(|l| l.span.file.to_string()),
        line: first_label.map(|l| l.span.line as u32),
        column: first_label.map(|l| l.span.column as u32),
      }
    })
    .collect();

  Ok(BuildReport { diagnostics, collections, errors: Vec::new() })
}

fn severity_label(s: duck_diagnostic::Severity) -> String {
  use duck_diagnostic::Severity;
  match s {
    Severity::Bug => "bug",
    Severity::Error => "error",
    Severity::Warning => "warning",
    Severity::Help => "help",
    Severity::Note => "note",
  }
  .to_string()
}
