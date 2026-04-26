use crate::pipeline::Transformer;
use crate::visit::{VisitFlow, Visitor, walk_mut};
use duck_md_parser::ast::*;
use std::path::PathBuf;

#[derive(Default)]
pub struct CodeImport {
  pub base_dir: Option<PathBuf>,
}

impl CodeImport {
  pub fn with_base_dir(p: impl Into<PathBuf>) -> Self {
    Self { base_dir: Some(p.into()) }
  }
}

impl Transformer for CodeImport {
  fn name(&self) -> &str {
    "code-import"
  }

  fn transform(&self, doc: &mut Document) {
    let mut v = Apply { base_dir: self.base_dir.clone() };
    for c in &mut doc.children {
      walk_mut(c, &mut v);
    }
  }
}

struct Apply {
  base_dir: Option<PathBuf>,
}

impl Visitor for Apply {
  fn visit_node(&mut self, node: &mut Node) -> VisitFlow {
    if let Node::CodeBlock(cb) = node
      && let Some(meta) = cb.meta.as_deref()
      && let Some((file, ranges)) = parse_file_meta(meta)
    {
      let path = match &self.base_dir {
        Some(b) => b.join(&file),
        None => PathBuf::from(&file),
      };
      if let Ok(content) = std::fs::read_to_string(&path) {
        cb.value = match ranges {
          Some(rs) => slice_lines(&content, &rs),
          None => content,
        };
      }
    }
    VisitFlow::Continue
  }
}

fn parse_file_meta(meta: &str) -> Option<(String, Option<Vec<(usize, usize)>>)> {
  for part in meta.split_whitespace() {
    if let Some(rest) = part.strip_prefix("file=") {
      let raw = rest.trim_matches(|c| c == '"' || c == '\'');
      if let Some((path, range)) = raw.split_once('{') {
        let range = range.trim_end_matches('}');
        let ranges = parse_ranges(range);
        return Some((path.to_string(), Some(ranges)));
      }
      return Some((raw.to_string(), None));
    }
  }
  None
}

fn parse_ranges(spec: &str) -> Vec<(usize, usize)> {
  let mut out = Vec::new();
  for token in spec.split(',') {
    let token = token.trim();
    if let Some((a, b)) = token.split_once('-') {
      if let (Ok(a), Ok(b)) = (a.trim().parse::<usize>(), b.trim().parse::<usize>()) {
        if a >= 1 && b >= a {
          out.push((a, b));
        }
      }
    } else if let Ok(n) = token.parse::<usize>() {
      if n >= 1 {
        out.push((n, n));
      }
    }
  }
  out
}

fn slice_lines(src: &str, ranges: &[(usize, usize)]) -> String {
  let lines: Vec<&str> = src.lines().collect();
  let mut out = String::new();
  for (a, b) in ranges {
    let start = a.saturating_sub(1);
    let end = (*b).min(lines.len());
    for (i, l) in lines.iter().enumerate().take(end).skip(start) {
      out.push_str(l);
      out.push('\n');
      let _ = i;
    }
  }
  out
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parse_file_meta_no_range() {
    assert_eq!(parse_file_meta("file=foo.rs"), Some(("foo.rs".into(), None)));
    assert_eq!(parse_file_meta("file=\"foo.rs\""), Some(("foo.rs".into(), None)));
  }

  #[test]
  fn parse_file_meta_with_range() {
    let (p, r) = parse_file_meta("file=foo.rs{1,3-5,8}").unwrap();
    assert_eq!(p, "foo.rs");
    assert_eq!(r, Some(vec![(1, 1), (3, 5), (8, 8)]));
  }

  #[test]
  fn slice_lines_picks_ranges() {
    let src = "a\nb\nc\nd\ne\n";
    let out = slice_lines(src, &[(2, 3)]);
    assert_eq!(out, "b\nc\n");
    let out = slice_lines(src, &[(1, 1), (4, 5)]);
    assert_eq!(out, "a\nd\ne\n");
  }
}
