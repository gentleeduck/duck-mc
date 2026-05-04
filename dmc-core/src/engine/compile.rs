use std::{path::Path, sync::Arc};

use dmc_codegen::{HtmlEmitter, MdxBodyEmitter, Walker};
use dmc_diagnostic::{
  Code,
  metadata::{Origin, SourceMeta},
};
use dmc_lexer::Lexer;
use dmc_parser::{Parser, ast::Document};
use dmc_transform::{CopyLinkedFilesOptions, MathEngine, PipelineConfig, PrettyCodeOptions};
use duck_diagnostic::DiagnosticEngine;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::engine::accumlator::Accumulator;

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(default)]
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
  /// Pretty-code highlighter config. `None` = bundled defaults
  /// (Catppuccin Latte/Mocha pair, dark primary, multi-mode CSS-vars
  /// output). `Some` = explicit theme spec.
  pub pretty_code: Option<PrettyCodeOptions>,
  /// LaTeX engine for `$...$` / `$$...$$`. `None` = KaTeX (slow, exact
  /// rehype-katex parity). `Some(MathEngine::Mathml)` = pulldown-latex
  /// MathML (fast, plainer visuals).
  pub math_engine: Option<MathEngine>,
}

impl Default for CompileConfig {
  fn default() -> Self {
    Self {
      markdown_gfm: true,
      emit_html: true,
      emit_body: true,
      mdx_output_format: None,
      mdx_minify: false,
      markdown_remark_plugins: vec![],
      markdown_rehype_plugins: vec![],
      mdx_remark_plugins: vec![],
      mdx_rehype_plugins: vec![],
      copy_linked_files: false,
      output_assets: None,
      output_base: None,
      pretty_code: None,
      math_engine: None,
    }
  }
}

impl CompileConfig {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn has_js_plugins(&self) -> bool {
    !self.effective_markdown_remark_plugins().is_empty()
      || !self.effective_mdx_remark_plugins().is_empty()
      || !self.effective_markdown_rehype_plugins().is_empty()
      || !self.effective_mdx_rehype_plugins().is_empty()
  }

  /// Plugin lists after stripping every JS plugin whose work is now done
  /// by an in-process transformer (pretty-code/shiki, math, emoji). Used
  /// both for the sidecar gate and for the request payload so the sidecar
  /// never duplicates work. When the matching feature is off, that
  /// plugin's name is left in the list and the sidecar runs it.
  pub fn effective_markdown_remark_plugins(&self) -> Vec<Value> {
    Self::filter_native_owned_remark(&self.markdown_remark_plugins)
  }

  pub fn effective_mdx_remark_plugins(&self) -> Vec<Value> {
    Self::filter_native_owned_remark(&self.mdx_remark_plugins)
  }

  pub fn effective_markdown_rehype_plugins(&self) -> Vec<Value> {
    Self::filter_native_owned_rehype(&self.markdown_rehype_plugins)
  }

  pub fn effective_mdx_rehype_plugins(&self) -> Vec<Value> {
    Self::filter_native_owned_rehype(&self.mdx_rehype_plugins)
  }

  fn filter_native_owned_remark(plugins: &[Value]) -> Vec<Value> {
    plugins.iter().filter(|p| !is_native_owned_remark(p)).cloned().collect()
  }

  fn filter_native_owned_rehype(plugins: &[Value]) -> Vec<Value> {
    plugins.iter().filter(|p| !is_native_owned_rehype(p)).cloned().collect()
  }

  /// Per-file compile config: turns off native HTML when sidecar will run.
  pub fn for_render(&self) -> Self {
    let mut c = self.clone();
    c.emit_html = !self.has_js_plugins();
    c
  }

  /// Build the [`PipelineConfig`] consumed by
  /// [`Pipeline::with_defaults_for`]. `path` is the compiled file's path,
  /// used to resolve relative asset paths in the `copy-linked-files`
  /// transformer.
  pub fn pipeline_config(&self, path: &Path) -> PipelineConfig {
    let copy_linked_files = if self.copy_linked_files
      && let (Some(assets), Some(public)) = (self.output_assets.as_ref(), self.output_base.as_ref())
    {
      Some(CopyLinkedFilesOptions {
        source_dir: path.parent().unwrap_or(Path::new(".")).to_path_buf(),
        assets_dir: assets.into(),
        public_base: public.clone(),
      })
    } else {
      None
    };
    PipelineConfig {
      markdown_gfm: Some(self.markdown_gfm),
      pretty_code: self.pretty_code.clone(),
      math_engine: self.math_engine,
      copy_linked_files,
    }
  }
}

/// Extract the plugin name from either the bare string form
/// (`"rehype-pretty-code"`) or the `[name, options]` array form used by
/// unified-style plugin configs.
fn plugin_name(plugin: &Value) -> Option<&str> {
  match plugin {
    Value::String(s) => Some(s.as_str()),
    Value::Array(a) => a.first().and_then(Value::as_str),
    _ => None,
  }
}

/// `true` when `plugin` is a remark-side plugin whose work an in-process
/// transformer now does. Stripped from the sidecar payload so the JS
/// plugin chain does not redo native work.
fn is_native_owned_remark(plugin: &Value) -> bool {
  let Some(name) = plugin_name(plugin) else { return false };
  match name {
    // GFM tables, strikethrough, autolinks, task lists are handled by
    // the dmc parser; remark-gfm in the sidecar is redundant.
    "remark-gfm" => true,
    "remark-math" => cfg!(feature = "math"),
    "remark-emoji" => cfg!(feature = "emoji"),
    _ => false,
  }
}

/// Same for rehype-side plugins.
fn is_native_owned_rehype(plugin: &Value) -> bool {
  let Some(name) = plugin_name(plugin) else { return false };
  match name {
    "rehype-pretty-code" | "shiki" => cfg!(feature = "pretty-code"),
    "rehype-katex" | "rehype-mathjax" => cfg!(feature = "math"),
    // Heading slugs + anchor links handled by the AutolinkHeadings
    // transformer in `Pipeline::with_defaults`.
    "rehype-slug" | "rehype-autolink-headings" => true,
    _ => false,
  }
}

pub struct Compiler;

impl Compiler {
  /// One-shot compile of `source` with the default pipeline. Use
  /// `compile_with_pipeline` for file-aware compilation with real spans.
  pub fn compile(source: &str, diag_engine: &mut DiagnosticEngine<Code>) -> CompileOutput {
    // FIX:
    Self::compile_with_pipeline(source, Path::new("."), &CompileConfig::new(), diag_engine)
  }

  /// Like [`compile`] with a caller-supplied pipeline + path for spans.
  pub fn compile_with_pipeline(
    source: &str,
    path: &Path,
    compile_cfg: &CompileConfig,
    diag_engine: &mut DiagnosticEngine<Code>,
  ) -> CompileOutput {
    // Each layer holds its own DiagnosticEngine, mirroring the Lexer pattern.
    let meta = Arc::from(SourceMeta {
      path: Arc::from(path.display().to_string()),
      version: 0, // TODO:
      origin: Origin::File(path.into()),
    });
    // Source-level math: rewrite `$...$` / `$$...$$` to `<MathMl/>` JSX
    // so the parser does not interpret `_` or `^` inside math as Markdown
    // emphasis markers.
    #[cfg(feature = "math")]
    let preprocessed = dmc_transform::Math::preprocess_source(source);
    #[cfg(feature = "math")]
    let source: &str = &preprocessed;
    let mut lexer = Lexer::new(source, meta.clone(), diag_engine);
    let _ = lexer.scan_tokens();

    let mut doc = {
      let mut parser = Parser::new(lexer.tokens, meta.clone(), diag_engine);
      parser.parse()
    };

    let pipeline_cfg = compile_cfg.pipeline_config(path);
    let pipeline = dmc_transform::Pipeline::with_defaults_for(&pipeline_cfg);

    pipeline.run(&mut doc, &meta, diag_engine);

    Self::finalize(source, doc, compile_cfg, diag_engine)
  }

  /// Pull frontmatter + imports/exports, render HTML + MDX body, derive
  /// excerpt / metadata / TOC, pack into a `CompileOutput`. Each sink
  /// owns a private `DiagnosticEngine` during the walk; we merge them
  /// into the caller's `diag_engine` after the walk completes (avoids
  /// `RefCell` overhead on every sink emit).
  fn finalize(
    source: &str,
    doc: Document,
    compile_cfg: &CompileConfig,
    diag_engine: &mut DiagnosticEngine<Code>,
  ) -> CompileOutput {
    let mut acc = Accumulator::new();
    let mut html_sink = if compile_cfg.emit_html { Some(HtmlEmitter::new()) } else { None };
    let mut body_sink = if compile_cfg.emit_body { Some(MdxBodyEmitter::new()) } else { None };

    let mut sinks: Vec<&mut dyn dmc_codegen::NodeSink> = Vec::with_capacity(3);
    sinks.push(&mut acc);
    if let Some(ref mut h) = html_sink {
      sinks.push(h);
    }
    if let Some(ref mut b) = body_sink {
      sinks.push(b);
    }

    Walker::new(&doc).walk(sinks.as_mut_slice());

    let (html, body) = match (html_sink, body_sink) {
      (Some(h), Some(b)) => {
        let (s, hd) = h.into_parts();
        let (m, bd) = b.into_parts();
        diag_engine.extend(hd);
        diag_engine.extend(bd);
        (s, m)
      },
      (Some(h), None) => {
        let (s, hd) = h.into_parts();
        diag_engine.extend(hd);
        (s, String::new())
      },
      (None, Some(b)) => {
        let (m, bd) = b.into_parts();
        diag_engine.extend(bd);
        (String::new(), m)
      },
      (None, None) => (String::new(), String::new()),
    };

    acc.into_compile_output(source, html, body, compile_cfg)
  }
}

/// Reading-time + word-count from plain text. `reading_time` in minutes,
/// ceil, min 1.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
  pub reading_time: u32,
  pub word_count: u32,
}

/// One TOC node. `url` is `#<heading-slug>`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TocItem {
  pub title: String,
  pub url: String,
  pub items: Vec<TocItem>,
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::json;

  #[test]
  fn empty_plugin_lists_no_sidecar() {
    let cfg = CompileConfig::default();
    assert!(!cfg.has_js_plugins());
  }

  #[test]
  fn arbitrary_remark_plugin_triggers_sidecar() {
    let mut cfg = CompileConfig::default();
    // Pick a plugin not covered by any native transformer.
    cfg.markdown_remark_plugins.push(json!("remark-frontmatter"));
    assert!(cfg.has_js_plugins());
  }

  #[test]
  fn remark_gfm_alone_skips_sidecar() {
    let mut cfg = CompileConfig::default();
    cfg.markdown_remark_plugins.push(json!("remark-gfm"));
    assert!(!cfg.has_js_plugins(), "dmc parser handles GFM natively");
  }

  #[test]
  fn rehype_slug_and_autolink_alone_skip_sidecar() {
    let mut cfg = CompileConfig::default();
    cfg.markdown_rehype_plugins.push(json!("rehype-slug"));
    cfg.markdown_rehype_plugins.push(json!(["rehype-autolink-headings", { "behavior": "wrap" }]));
    assert!(!cfg.has_js_plugins(), "AutolinkHeadings transformer handles slug + anchor natively");
  }

  #[cfg(feature = "math")]
  #[test]
  fn remark_math_alone_with_native_skips_sidecar() {
    let mut cfg = CompileConfig::default();
    cfg.markdown_remark_plugins.push(json!("remark-math"));
    cfg.markdown_rehype_plugins.push(json!(["rehype-katex", { "errorColor": "red" }]));
    assert!(!cfg.has_js_plugins(), "native math should absorb remark-math + rehype-katex");
  }

  #[cfg(feature = "emoji")]
  #[test]
  fn remark_emoji_alone_with_native_skips_sidecar() {
    let mut cfg = CompileConfig::default();
    cfg.markdown_remark_plugins.push(json!("remark-emoji"));
    assert!(!cfg.has_js_plugins(), "native emoji should absorb remark-emoji");
  }

  #[cfg(feature = "pretty-code")]
  #[test]
  fn rehype_pretty_code_alone_with_native_skips_sidecar() {
    let mut cfg = CompileConfig::default();
    cfg.markdown_rehype_plugins.push(json!("rehype-pretty-code"));
    cfg.mdx_rehype_plugins.push(json!(["rehype-pretty-code", { "theme": "github-dark" }]));
    cfg.mdx_rehype_plugins.push(json!("shiki"));
    assert!(!cfg.has_js_plugins(), "native should absorb rehype-pretty-code/shiki");
  }

  #[cfg(feature = "pretty-code")]
  #[test]
  fn other_rehype_plugin_still_triggers_sidecar_even_with_native() {
    let mut cfg = CompileConfig::default();
    cfg.markdown_rehype_plugins.push(json!("rehype-pretty-code"));
    // Any rehype plugin not absorbed by a native transformer keeps the
    // sidecar alive. Pick something that no current native pass owns.
    cfg.markdown_rehype_plugins.push(json!("rehype-external-links"));
    assert!(cfg.has_js_plugins());
  }

  #[cfg(not(feature = "pretty-code"))]
  #[test]
  fn pretty_code_feature_off_means_rehype_pretty_code_routes_to_sidecar() {
    let mut cfg = CompileConfig::default();
    cfg.markdown_rehype_plugins.push(json!("rehype-pretty-code"));
    assert!(cfg.has_js_plugins());
  }
}

/// Compiled `.mdx` output. Every field is always populated; serialised
/// camelCase for JS parity.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
