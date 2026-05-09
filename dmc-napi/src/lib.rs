#![deny(clippy::all)]

use napi::bindgen_prelude::*;
use napi_derive::napi;
use serde_json::Value;
use std::path::PathBuf;

use dmc::Engine;
use dmc::engine::collection::Collection as CollectionDef;
use dmc::engine::compile::{CompileConfig, Compiler};
use dmc::engine::config::EngineConfig;
use dmc_diagnostic::Code;
use duck_diagnostic::DiagnosticEngine;

#[napi]
pub fn compile(source: String) -> Result<Value> {
  let mut diag = DiagnosticEngine::<Code>::new();
  let out = Compiler::compile(&source, &mut diag);
  serde_json::to_value(&out).map_err(|e| Error::from_reason(e.to_string()))
}

/// Render a LaTeX fragment to KaTeX HTML via the embedded KaTeX engine.
/// Output matches the JS chain `rehype-katex` byte-for-byte. Pair with
/// the standard `katex.min.css` for glyph rendering.
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
pub fn compile_many(sources: Vec<String>) -> Result<Vec<Value>> {
  let mut diag = DiagnosticEngine::<Code>::new();
  sources
    .into_iter()
    .map(|s| {
      let out = Compiler::compile(&s, &mut diag);
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
  /// Bypass the plugin gate for every plugin: every JS plugin runs
  /// in the sidecar, every native transformer is dropped.
  pub force_sidecar: Option<bool>,
  /// Per-plugin sidecar preference. Names listed here run in the
  /// sidecar; the matching native transformer is dropped from the
  /// pipeline. Names dmc recognises:
  ///   "remark-gfm", "remark-math", "remark-emoji",
  ///   "rehype-pretty-code", "shiki",
  ///   "rehype-katex", "rehype-mathjax",
  ///   "rehype-slug", "rehype-autolink-headings",
  ///   "mermaid", "rehype-mermaid", "remark-mermaid"
  pub prefer_sidecar: Option<Vec<String>>,
  /// Mermaid render config. Free-form JSON deserialised into
  /// `MermaidOptions`:
  ///   theme:               string | { [mode: string]: string }
  ///   config:              any (forwarded to mmdc --configFile)
  ///   backgroundColor:     string ("transparent" by default)
  ///   htmlLabels:          boolean (default false)
  ///   responsiveSvg:       boolean (default true)
  ///   centerLabels:        boolean (default true)
  ///   outputDir:           string (disk SVG cache)
  ///   puppeteerConfigFile: string
  pub mermaid: Option<Value>,
  /// Pretty-code config. Free-form JSON deserialised into
  /// `PrettyCodeOptions`:
  ///   theme:               string | { [mode: string]: string }
  ///   defaultMode:         string
  ///   keepRawString:       boolean (default true)
  ///   fragmentWrapper:     boolean (default true)
  ///   lineClass:           string ("line" by default)
  ///   highlightedLineAttr: string ("data-dmc-line-highlighted" by default)
  ///   defaultLanguage:     string ("plaintext" by default)
  ///   fallbackToPlaintext: boolean (default true)
  ///   renderTitle:         boolean (default true)
  ///   includeDataLanguage: boolean (default true)
  ///   skipLanguages:       string[]
  ///   tabSize:             number
  pub pretty_code: Option<Value>,
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
  /// Optional follow-up help text (e.g. `bundled themes: …`).
  pub help: Option<String>,
  /// First label's source-file path (when present). Lets the
  /// JS-side formatter prefix `path:line:col` like rustc does.
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
    pretty_code: input
      .pretty_code
      .as_ref()
      .map(|v| {
        serde_json::from_value::<dmc::PrettyCodeOptions>(v.clone())
          .map_err(|e| Error::from_reason(format!("invalid prettyCode config: {e}")))
      })
      .transpose()?,
    mermaid: input
      .mermaid
      .as_ref()
      .map(|v| {
        serde_json::from_value::<dmc::MermaidOptions>(v.clone())
          .map_err(|e| Error::from_reason(format!("invalid mermaid config: {e}")))
      })
      .transpose()?,
    math_engine: None,
    force_sidecar: input.force_sidecar.unwrap_or(false),
    prefer_sidecar: input.prefer_sidecar.unwrap_or_default(),
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
      })
      .collect(),
    compile,
  };

  let mut diag = DiagnosticEngine::<Code>::new();
  // `Engine::run` now returns `DiagResult`. The
  // diagnostic carries `code` + `message`; we lose `help` / labels at
  // the napi boundary because napi-rs needs a `String` error. The
  // structured detail still ships back to JS via
  // `BuildReport.diagnostics`, so the message-only conversion here is
  // fine for the abort path.
  if let Err(d) = Engine::run(&cfg, None, &mut diag) {
    use duck_diagnostic::DiagnosticCode;
    return Err(Error::from_reason(format!("{}: {}", d.code.code(), d.message)));
  }

  // Surface enough collection metadata for the JS-side `build()` wrapper
  // to do its in-process unified pipeline pass. Engine writes one JSON
  // file per collection at `<output_dir>/<name>.json`; we count records
  // by re-reading the file (cheap relative to the build itself).
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
