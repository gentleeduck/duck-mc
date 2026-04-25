use crate::pipeline::Transformer;
use crate::visit::{walk_mut, VisitFlow, Visitor};
use duck_md_ast::*;

#[derive(Default)]
pub struct BareUrlAutolink;

impl Transformer for BareUrlAutolink {
  fn name(&self) -> &str {
    "bare-url-autolink"
  }
  fn transform(&self, doc: &mut Document) {
    let mut v = Apply;
    for c in &mut doc.children {
      walk_mut(c, &mut v);
    }
  }
}

struct Apply;

impl Visitor for Apply {
  fn visit_node(&mut self, node: &mut Node) -> VisitFlow {
    match node {
      Node::Paragraph(p) => p.children = rewrite_children(std::mem::take(&mut p.children)),
      Node::Heading(h) => h.children = rewrite_children(std::mem::take(&mut h.children)),
      Node::Bold(i) | Node::Italic(i) | Node::Strikethrough(i) => {
        i.children = rewrite_children(std::mem::take(&mut i.children));
      },
      _ => {},
    }
    VisitFlow::Continue
  }
}

fn rewrite_children(nodes: Vec<Node>) -> Vec<Node> {
  let mut out = Vec::new();
  for n in nodes {
    if let Node::Text(t) = &n {
      let pieces = split_by_url(&t.value);
      if pieces.len() == 1 {
        out.push(n.clone());
        continue;
      }
      for piece in pieces {
        match piece {
          Piece::Text(s) if !s.is_empty() => out.push(Node::Text(Text {
            value: s,
            span: default_span(),
          })),
          Piece::Text(_) => {},
          Piece::Url(url) => out.push(Node::Link(Link {
            href: url.clone(),
            title: None,
            children: vec![Node::Text(Text {
              value: url,
              span: default_span(),
            })],
            span: default_span(),
          })),
        }
      }
    } else {
      out.push(n);
    }
  }
  out
}

enum Piece {
  Text(String),
  Url(String),
}

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
    // URL ends at first whitespace, ), or end
    let url_end = after
      .find(|c: char| c.is_whitespace() || c == ')' || c == '<' || c == '>')
      .unwrap_or(after.len());
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
