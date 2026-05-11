//! Bare-URL autolinker. See `transformers/bare-url.md` for full docs.

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
        // No URL found if every piece is a Text (just the original
        // string round-tripping). Otherwise rewrite into the
        // text+link mix.
        let any_url = pieces.iter().any(|p| matches!(p, Piece::Url(_) | Piece::Display(_)));
        if !any_url {
          out.push(n.clone());
          continue;
        }
        let span = t.span.clone();
        let mut iter = pieces.into_iter().peekable();
        while let Some(piece) = iter.next() {
          match piece {
            Piece::Text(s) if !s.is_empty() => out.push(Node::Text(Text { value: s, span: span.clone() })),
            Piece::Text(_) => {},
            Piece::Url(href) => {
              let display = match iter.peek() {
                Some(Piece::Display(_)) => match iter.next() {
                  Some(Piece::Display(d)) => d,
                  _ => href.clone(),
                },
                _ => href.clone(),
              };
              out.push(Node::Link(Link {
                href,
                title: None,
                children: vec![Node::Text(Text { value: display, span: span.clone() })],
                span: span.clone(),
              }));
            },
            Piece::Display(d) => {
              // Stray Display without preceding Url -- emit as text.
              if !d.is_empty() {
                out.push(Node::Text(Text { value: d, span: span.clone() }));
              }
            },
          }
        }
      } else {
        out.push(n);
      }
    }
    out
  }

  /// Split `s` into alternating `Text` / `Url` pieces around GFM
  /// autolink runs: `http(s)://...` plus `www....`. URL boundary is
  /// whitespace, `<`, or unbalanced `)`. Trailing `?!.,:*_~` is
  /// trimmed as sentence punctuation; trailing `&entity;` is also
  /// stripped because GFM treats the entity ref as following text.
  fn split_by_url(s: &str) -> Vec<Piece> {
    fn next_url_match(rest: &str) -> Option<(usize, &'static str)> {
      // Find the earliest position where one of the GFM autolink
      // prefixes starts at a valid boundary (start of string or
      // preceded by a non-alphanumeric / `_`).
      let bytes = rest.as_bytes();
      let mut best: Option<(usize, &'static str)> = None;
      for prefix in ["http://", "https://", "www."] {
        if let Some(idx) = rest.find(prefix) {
          let ok_boundary =
            idx == 0 || matches!(bytes.get(idx - 1).copied(), Some(b) if !b.is_ascii_alphanumeric() && b != b'_');
          if !ok_boundary {
            continue;
          }
          if best.is_none_or(|(b, _)| idx < b) {
            best = Some((idx, prefix));
          }
        }
      }
      best
    }
    fn url_body_end(after: &str) -> usize {
      after.find(|c: char| c.is_whitespace() || c == '<').unwrap_or(after.len())
    }
    fn trim_trailing(s: &str) -> (&str, &str) {
      let bytes = s.as_bytes();
      let mut end = bytes.len();
      loop {
        if end == 0 {
          break;
        }
        let last = bytes[end - 1];
        // Strip trailing sentence punctuation.
        if matches!(last, b'?' | b'!' | b'.' | b',' | b':' | b'*' | b'_' | b'~') {
          end -= 1;
          continue;
        }
        // Strip an unmatched `)` (more closes than opens in the
        // current URL body).
        if last == b')' {
          let opens = bytes[..end].iter().filter(|&&b| b == b'(').count();
          let closes = bytes[..end].iter().filter(|&&b| b == b')').count();
          if closes > opens {
            end -= 1;
            continue;
          }
        }
        // Strip a trailing `&entity;` (entity refs render as following
        // text per GFM autolink rule).
        if last == b';'
          && let Some(amp) = bytes[..end - 1].iter().rposition(|&b| b == b'&')
        {
          let inner = &bytes[amp + 1..end - 1];
          if !inner.is_empty() && inner.iter().all(|&b| b.is_ascii_alphanumeric()) {
            end = amp;
            continue;
          }
        }
        break;
      }
      (&s[..end], &s[end..])
    }

    let mut out = Vec::new();
    let mut rest = s;
    while let Some((idx, prefix)) = next_url_match(rest) {
      let before = &rest[..idx];
      let after = &rest[idx..];
      let url_end = url_body_end(after);
      let raw = &after[..url_end];
      let (url, trailing_punct) = trim_trailing(raw);
      // GFM: `www.` autolinks require a `.` in the body after the prefix
      // (the prefix itself ends with `.`). `trim_trailing` can shave the
      // body down to (or below) the prefix length -- eg `www.` alone, or
      // `www.` followed only by trailing punctuation -- so look up the
      // body fallibly instead of slicing `url[prefix.len()..]` blindly.
      if prefix == "www." && !url.get(prefix.len()..).is_some_and(|body| body.contains('.')) {
        out.push(Piece::Text(format!("{}{}", before, prefix)));
        rest = &after[prefix.len()..];
        continue;
      }
      if url.is_empty() {
        out.push(Piece::Text(before.to_string()));
        rest = &after[1..];
        continue;
      }
      if !before.is_empty() {
        out.push(Piece::Text(before.to_string()));
      }
      let href = if prefix == "www." { format!("http://{}", url) } else { url.to_string() };
      out.push(Piece::Url(href));
      out.push(Piece::Display(url.to_string()));
      if !trailing_punct.is_empty() {
        out.push(Piece::Text(trailing_punct.to_string()));
      }
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
  /// Resolved link destination (with `http://` prefix injected for
  /// `www.` matches). Always immediately followed by `Display`.
  Url(String),
  /// Visible text inside the synthesized `<a>` (matches the raw
  /// autolink slice in the source, eg `www.commonmark.org`).
  Display(String),
}
