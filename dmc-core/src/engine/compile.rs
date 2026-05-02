use std::{path::Path, sync::Arc};

use dmc_diagnostic::{
  Code,
  metadata::{Origin, SourceMeta},
};
use dmc_lexer::Lexer;
use dmc_parser::{
  Parser,
  ast::{Document, Node},
};
use duck_diagnostic::DiagnosticEngine;
use serde::{Deserialize, Serialize};
use serde_json::Value;

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

  /// Derive a per-file compile config: turn off native HTML when sidecar will run.
  pub fn for_render(&self) -> Self {
    let mut c = self.clone();
    c.emit_html = !self.has_js_plugins();
    c
  }
}

pub struct Compiler;

impl Compiler {
  /// One-shot compile of `source` using the default transform pipeline.
  /// Source identity defaults to `Origin::Inline("<inline>")` - use the
  /// `engine` module for file-aware compilation with real spans.
  pub fn compile(source: &str, diag_engine: &mut DiagnosticEngine<Code>) -> CompileOutput {
    // FIX:
    Self::compile_with_pipeline(source, Path::new("."), &CompileConfig::new(), diag_engine)
  }

  /// Like [`compile`] but lets the caller supply a custom pipeline. Diagnostics
  /// emitted by each layer are currently dropped at the boundary; consumers
  /// that need them (LSP / CLI) should drive the layers themselves.
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

    Self::finalize(source, doc, compile_cfg)
  }

  /// Pull frontmatter + imports/exports off the AST, render HTML and MDX body,
  /// derive excerpt / metadata / TOC, and pack the result into a CompileOutput.
  fn finalize(source: &str, doc: Document, compile_cfg: &CompileConfig) -> CompileOutput {
    let mut frontmatter = serde_json::Value::Null;
    let mut frontmatter_raw = String::new();
    let mut imports = Vec::new();
    let mut exports = Vec::new();

    for child in &doc.children {
      match child {
        Node::Frontmatter(f) => {
          frontmatter = serde_yaml::from_str::<serde_json::Value>(&f.raw).unwrap_or(serde_json::Value::Null);
          frontmatter_raw = f.raw.clone();
        },
        Node::Import(i) => imports.push(i.raw.clone()),
        Node::Export(e) => exports.push(e.raw.clone()),
        _ => {},
      }
    }

    let content = Self::strip_frontmatter(source).to_string();
    let html = if compile_cfg.emit_html { dmc_codegen::render_html(&doc) } else { String::new() };
    let body = if compile_cfg.emit_body { dmc_codegen::render_mdx_body(&doc) } else { String::new() };
    let plain = Self::plain_text(&doc);
    let excerpt = Self::build_excerpt(&plain, 260);
    let metadata = Self::build_metadata(&plain);
    let toc = Self::build_toc(&doc);

    CompileOutput { frontmatter, frontmatter_raw, content, html, body, excerpt, metadata, toc, imports, exports }
  }

  /// Return `source` with a leading `---...---` YAML frontmatter block removed
  /// (BOM tolerant). Used so `CompileOutput.content` is "the body the author
  /// actually wrote" without the metadata header.
  fn strip_frontmatter(source: &str) -> &str {
    let s = source.trim_start_matches('\u{feff}');
    if !s.starts_with("---") {
      return source;
    }
    // find the next "---" on its own line
    let after = &s[3..];
    if let Some(end) = after.find("\n---") {
      let rest_start = 3 + end + 4; // 3 dashes + \n--- = 4 chars after end
      // skip optional newline after the closing ---
      let rest = &s[rest_start..];
      let rest = rest.trim_start_matches('\n');
      return rest;
    }
    source
  }

  /// Flatten the document to a single string of human-readable text.
  /// Backs the excerpt, metadata, and TOC builders below.
  fn plain_text(doc: &Document) -> String {
    let mut out = String::new();
    for n in &doc.children {
      Self::plain_node(n, &mut out);
    }
    out
  }

  /// Recursive helper for [`plain_text`]. Skips JSX expressions, attrs, and
  /// breaks; keeps Text / InlineCode / CodeBlock content + Image alt text.
  fn plain_node(n: &Node, out: &mut String) {
    match n {
      Node::Text(t) => out.push_str(&t.value),
      Node::Heading(h) => {
        for c in &h.children {
          Self::plain_node(c, out);
        }
        out.push('\n');
      },
      Node::Paragraph(p) => {
        for c in &p.children {
          Self::plain_node(c, out);
        }
        out.push('\n');
      },
      Node::Bold(i) | Node::Italic(i) | Node::Strikethrough(i) => {
        for c in &i.children {
          Self::plain_node(c, out);
        }
      },
      Node::InlineCode(c) => out.push_str(&c.value),
      Node::CodeBlock(c) => out.push_str(&c.value),
      Node::Link(l) => {
        for c in &l.children {
          Self::plain_node(c, out);
        }
      },
      Node::Image(i) => out.push_str(&i.alt),
      Node::List(l) => {
        for c in &l.children {
          Self::plain_node(c, out);
        }
      },
      Node::ListItem(li) => {
        for c in &li.children {
          Self::plain_node(c, out);
        }
      },
      Node::TaskListItem(t) => {
        for c in &t.children {
          Self::plain_node(c, out);
        }
      },
      Node::Blockquote(b) => {
        for c in &b.children {
          Self::plain_node(c, out);
        }
      },
      Node::Table(t) => {
        for row in &t.children {
          for cell in &row.cells {
            for c in &cell.children {
              Self::plain_node(c, out);
            }
            out.push(' ');
          }
          out.push('\n');
        }
      },
      Node::JsxElement(e) => {
        for c in &e.children {
          Self::plain_node(c, out);
        }
      },
      Node::JsxFragment(f) => {
        for c in &f.children {
          Self::plain_node(c, out);
        }
      },
      _ => {},
    }
  }

  /// Collapse whitespace and truncate to at most `max` chars, appending `...`
  /// when the original exceeded the limit. Char-aware (multibyte safe).
  fn build_excerpt(plain: &str, max: usize) -> String {
    let s: String = plain.split_whitespace().collect::<Vec<_>>().join(" ");
    if s.chars().count() <= max {
      return s;
    }
    let truncated: String = s.chars().take(max).collect();
    format!("{}...", truncated.trim_end())
  }

  /// Compute word count and reading time from the plain-text body.
  /// Reading rate is hardcoded at 200 wpm, rounded up, minimum 1 minute.
  fn build_metadata(plain: &str) -> Metadata {
    let words = plain.split_whitespace().count() as u32;
    let reading = ((words as f32) / 200.0).ceil() as u32;
    Metadata { word_count: words, reading_time: reading.max(1) }
  }

  /// Walk top-level Headings, slug each, then nest into a tree by level.
  fn build_toc(doc: &Document) -> Vec<TocItem> {
    // collect (level, title, id) in order
    let mut flat: Vec<(u8, String, String)> = Vec::new();
    for n in &doc.children {
      if let Node::Heading(h) = n {
        let mut s = String::new();
        for c in &h.children {
          Self::plain_node(c, &mut s);
        }
        flat.push((h.level, s.trim().to_string(), h.slug()));
      }
    }
    Self::nest(&flat)
  }

  /// Convert a flat `(level, title, slug)` list into a hierarchical TocItem
  /// tree. A level stack tracks the current ancestry; new headings either nest
  /// under the last open parent or pop back to an earlier ancestor.
  fn nest(items: &[(u8, String, String)]) -> Vec<TocItem> {
    let mut roots: Vec<TocItem> = Vec::new();
    // index path into the children tree, parallel with the level stack
    let mut path: Vec<usize> = Vec::new();
    let mut levels: Vec<u8> = Vec::new();
    for (level, title, id) in items {
      let item = TocItem { title: title.clone(), url: format!("#{}", id), items: Vec::new() };
      // pop until top has lower level
      while let Some(top) = levels.last() {
        if *top >= *level {
          levels.pop();
          path.pop();
        } else {
          break;
        }
      }
      // navigate to insertion list
      let parent_list: &mut Vec<TocItem> = if path.is_empty() {
        &mut roots
      } else {
        let mut node = &mut roots[path[0]];
        for idx in &path[1..] {
          node = &mut node.items[*idx];
        }
        &mut node.items
      };
      parent_list.push(item);
      let new_idx = parent_list.len() - 1;
      path.push(new_idx);
      levels.push(*level);
    }
    roots
  }
}

/// Reading-time + word-count summary derived from the document's plain text.
/// `reading_time` is in minutes, rounded up, minimum 1.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
  pub reading_time: u32,
  pub word_count: u32,
}

/// One node of the table-of-contents tree. `url` is `#<heading-slug>`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TocItem {
  pub title: String,
  pub url: String,
  pub items: Vec<TocItem>,
}

/// Everything a downstream consumer (docs site / SSG / LSP) needs from one
/// compiled `.mdx` document. Every field is always populated; serialised in
/// camelCase for JS-side parity.
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
