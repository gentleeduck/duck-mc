use duck_diagnostic::DiagnosticEngine;
use duck_md_diagnostic::metadata::Origin;
use duck_md_diagnostic::metadata::SourceMeta;
use duck_md_lexer::Lexer;
use duck_md_parser::Parser;
use duck_md_parser::ast::*;
use duck_md_transform::Pipeline;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
  pub reading_time: u32,
  pub word_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TocItem {
  pub title: String,
  pub url: String,
  pub items: Vec<TocItem>,
}

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

pub fn compile(source: &str) -> CompileOutput {
  compile_with_pipeline(source, &Pipeline::with_defaults())
}

pub fn compile_with_pipeline(source: &str, pipeline: &Pipeline) -> CompileOutput {
  // Each layer holds its own DiagnosticEngine, mirroring the Lexer pattern.
  let meta = Arc::from(SourceMeta {
    path: Arc::from("<inline>"),
    version: 0,
    origin: Origin::Inline("<inline>"),
  });
  let lex_engine = RefCell::new(DiagnosticEngine::new());
  let mut lexer = Lexer::new(source, meta.clone(), lex_engine.borrow_mut());
  let _ = lexer.scan_tokens();
  let tokens = std::mem::take(&mut lexer.tokens);
  drop(lexer);

  let parse_engine = RefCell::new(DiagnosticEngine::new());
  let mut doc = {
    let mut parser = Parser::new(tokens, meta.clone(), parse_engine.borrow_mut());
    parser.parse()
  };

  let transform_engine = RefCell::new(DiagnosticEngine::new());
  pipeline.run(&mut doc, &meta, transform_engine.borrow_mut());

  // Diagnostics are currently dropped at the compile() boundary. Wire into
  // CompileOutput when consumers need them (LSP, CLI error-reporting, etc.).
  let _ = (lex_engine, parse_engine, transform_engine);

  finalize(source, doc)
}

fn finalize(source: &str, doc: Document) -> CompileOutput {
  let mut frontmatter = serde_json::Value::Null;
  let mut frontmatter_raw = String::new();
  let mut imports = Vec::new();
  let mut exports = Vec::new();

  for child in &doc.children {
    match child {
      Node::Frontmatter(f) => {
        frontmatter =
          serde_yaml::from_str::<serde_json::Value>(&f.raw).unwrap_or(serde_json::Value::Null);
        frontmatter_raw = f.raw.clone();
      },
      Node::Import(i) => imports.push(i.raw.clone()),
      Node::Export(e) => exports.push(e.raw.clone()),
      _ => {},
    }
  }

  let content = strip_frontmatter(source).to_string();
  let html = duck_md_codegen::render_html(&doc);
  let body = duck_md_codegen::render_mdx_body(&doc);
  let plain = plain_text(&doc);
  let excerpt = build_excerpt(&plain, 260);
  let metadata = build_metadata(&plain);
  let toc = build_toc(&doc);

  CompileOutput {
    frontmatter,
    frontmatter_raw,
    content,
    html,
    body,
    excerpt,
    metadata,
    toc,
    imports,
    exports,
  }
}

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

fn plain_text(doc: &Document) -> String {
  let mut out = String::new();
  for n in &doc.children {
    plain_node(n, &mut out);
  }
  out
}

fn plain_node(n: &Node, out: &mut String) {
  match n {
    Node::Text(t) => out.push_str(&t.value),
    Node::Heading(h) => {
      for c in &h.children {
        plain_node(c, out);
      }
      out.push('\n');
    },
    Node::Paragraph(p) => {
      for c in &p.children {
        plain_node(c, out);
      }
      out.push('\n');
    },
    Node::Bold(i) | Node::Italic(i) | Node::Strikethrough(i) => {
      for c in &i.children {
        plain_node(c, out);
      }
    },
    Node::InlineCode(c) => out.push_str(&c.value),
    Node::CodeBlock(c) => out.push_str(&c.value),
    Node::Link(l) => {
      for c in &l.children {
        plain_node(c, out);
      }
    },
    Node::Image(i) => out.push_str(&i.alt),
    Node::List(l) => {
      for c in &l.children {
        plain_node(c, out);
      }
    },
    Node::ListItem(li) => {
      for c in &li.children {
        plain_node(c, out);
      }
    },
    Node::TaskListItem(t) => {
      for c in &t.children {
        plain_node(c, out);
      }
    },
    Node::Blockquote(b) => {
      for c in &b.children {
        plain_node(c, out);
      }
    },
    Node::Table(t) => {
      for row in &t.children {
        for cell in &row.cells {
          for c in &cell.children {
            plain_node(c, out);
          }
          out.push(' ');
        }
        out.push('\n');
      }
    },
    Node::JsxElement(e) => {
      for c in &e.children {
        plain_node(c, out);
      }
    },
    Node::JsxFragment(f) => {
      for c in &f.children {
        plain_node(c, out);
      }
    },
    _ => {},
  }
}

fn build_excerpt(plain: &str, max: usize) -> String {
  let s: String = plain.split_whitespace().collect::<Vec<_>>().join(" ");
  if s.chars().count() <= max {
    return s;
  }
  let truncated: String = s.chars().take(max).collect();
  format!("{}…", truncated.trim_end())
}

fn build_metadata(plain: &str) -> Metadata {
  let words = plain.split_whitespace().count() as u32;
  let reading = ((words as f32) / 200.0).ceil() as u32;
  Metadata { word_count: words, reading_time: reading.max(1) }
}

fn build_toc(doc: &Document) -> Vec<TocItem> {
  // collect (level, title, id) in order
  let mut flat: Vec<(u8, String, String)> = Vec::new();
  for n in &doc.children {
    if let Node::Heading(h) = n {
      let mut s = String::new();
      for c in &h.children {
        plain_node(c, &mut s);
      }
      flat.push((h.level, s.trim().to_string(), h.slug()));
    }
  }
  nest(&flat)
}

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
