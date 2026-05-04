use crate::pipeline::Transformer;
use crate::visit::{NodeAction, Visitor, walk_root};
use dmc_diagnostic::Code;
use dmc_diagnostic::metadata::SourceMeta;
use dmc_parser::ast::*;

/// Wrap bare `http(s)://...` substrings in `Text` nodes with synthesised
/// `Link` nodes. Scans `Paragraph`, `Heading`, and inline emphasis containers.
#[derive(Default)]
pub struct BareUrlAutolink;

impl Transformer for BareUrlAutolink {
  fn name(&self) -> &str {
    "bare-url-autolink"
  }
  fn transform(
    &self,
    doc: &mut Document,
    _meta: &SourceMeta,
    _diag_engine: &mut duck_diagnostic::DiagnosticEngine<Code>,
  ) {
    let mut v = Apply;
    walk_root(&mut doc.children, &mut v);
  }
}

struct Apply;

impl Apply {
  /// Expand any `Text` whose value contains a URL into `[Text, Link, Text,
  /// ...]` pieces. Non-Text nodes pass through.
  fn rewrite_children(nodes: Vec<Node>) -> Vec<Node> {
    let mut out = Vec::new();
    for n in nodes {
      if let Node::Text(t) = &n {
        let pieces = Self::split_by_url(&t.value);
        if pieces.len() == 1 {
          out.push(n.clone());
          continue;
        }
        let span = t.span.clone();
        for piece in pieces {
          match piece {
            Piece::Text(s) if !s.is_empty() => out.push(Node::Text(Text { value: s, span: span.clone() })),
            Piece::Text(_) => {},
            Piece::Url(url) => out.push(Node::Link(Link {
              href: url.clone(),
              title: None,
              children: vec![Node::Text(Text { value: url, span: span.clone() })],
              span: span.clone(),
            })),
          }
        }
      } else {
        out.push(n);
      }
    }
    out
  }

  /// Split `s` into alternating `Text` / `Url` pieces around `http(s)://`
  /// runs. URL boundary is whitespace, `)`, `<`, or `>`.
  fn split_by_url(s: &str) -> Vec<Piece> {
    let mut out = Vec::new();
    let mut rest = s;
    while let Some(idx) = rest.find("http") {
      let before = &rest[..idx];
      let after = &rest[idx..];
      if !(after.starts_with("http://") || after.starts_with("https://")) {
        out.push(Piece::Text(format!("{}{}", before, &rest[idx..idx + 1])));
        rest = &rest[idx + 1..];
        continue;
      }
      let url_end = after.find(|c: char| c.is_whitespace() || c == ')' || c == '<' || c == '>').unwrap_or(after.len());
      let url = &after[..url_end];
      if !before.is_empty() {
        out.push(Piece::Text(before.to_string()));
      }
      out.push(Piece::Url(url.to_string()));
      rest = &after[url_end..];
    }
    if !rest.is_empty() {
      out.push(Piece::Text(rest.to_string()));
    }
    if out.is_empty() {
      out.push(Piece::Text(String::new()));
    }
    out
  }
}

impl Visitor for Apply {
  fn visit_node(&mut self, node: &mut Node) -> NodeAction {
    match node {
      Node::Paragraph(p) => p.children = Self::rewrite_children(std::mem::take(&mut p.children)),
      Node::Heading(h) => h.children = Self::rewrite_children(std::mem::take(&mut h.children)),
      Node::Bold(i) | Node::Italic(i) | Node::Strikethrough(i) => {
        i.children = Self::rewrite_children(std::mem::take(&mut i.children));
      },
      _ => {},
    }
    NodeAction::Keep
  }
}

enum Piece {
  Text(String),
  Url(String),
}
