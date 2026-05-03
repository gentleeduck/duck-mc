use std::{path::Path, sync::Arc};

use dmc_codegen::{HtmlEmitter, MdxBodyEmitter, Walker};
use dmc_diagnostic::{
  Code,
  metadata::{Origin, SourceMeta},
};
use dmc_lexer::Lexer;
use dmc_parser::{Parser, ast::Document};
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
    }
  }
}

impl CompileConfig {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn has_js_plugins(&self) -> bool {
    let any_filled = |v: &Vec<Value>| !v.is_empty();
    any_filled(&self.markdown_remark_plugins)
      || any_filled(&self.markdown_rehype_plugins)
      || any_filled(&self.mdx_remark_plugins)
      || any_filled(&self.mdx_rehype_plugins)
  }

  /// Per-file compile config: turns off native HTML when sidecar will run.
  pub fn for_render(&self) -> Self {
    let mut c = self.clone();
    c.emit_html = !self.has_js_plugins();
    c
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
    let mut lexer = Lexer::new(source, meta.clone(), diag_engine);
    let _ = lexer.scan_tokens();

    let mut doc = {
      let mut parser = Parser::new(lexer.tokens, meta.clone(), diag_engine);
      parser.parse()
    };

    let mut pipeline = dmc_transform::Pipeline::with_defaults();

    // TODO: refactor the transfomers below later on
    if !compile_cfg.markdown_gfm {
      pipeline = pipeline.add(dmc_transform::DisableGfm);
    }

    if compile_cfg.copy_linked_files && compile_cfg.output_assets.is_some() && compile_cfg.output_base.is_some() {
      pipeline = pipeline.add(dmc_transform::CopyLinkedFiles::new(
        path.parent().unwrap_or(std::path::Path::new(".")).to_path_buf(),
        compile_cfg.output_assets.clone().unwrap().into(),
        compile_cfg.output_base.clone().unwrap(),
      ));
    }

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
