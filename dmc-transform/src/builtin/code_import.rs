use crate::pipeline::Transformer;
use crate::visit::{NodeAction, Visitor, walk_root};
use dmc_diagnostic::Code;
use dmc_diagnostic::metadata::{Origin, SourceMeta};
use dmc_parser::ast::*;
use duck_diagnostic::{Diagnostic, Label};
use std::path::PathBuf;
use std::sync::Arc;

/// Resolve `file=path[{ranges}]` directives inside fenced code-block info
/// strings. Reads the named file from disk, replacing the block's body with
/// the file contents (optionally sliced by 1-based line ranges).
///
/// Path resolution order (first hit wins):
/// 1. explicit `base_dir` (set via [`CodeImport::with_base_dir`])
/// 2. parent dir of `meta.origin` if it's [`Origin::File`]
/// 3. cwd — emits a [`Code::BaseDirNotFound`] warning, paths must be absolute
pub struct CodeImport {
  pub base_dir: Option<PathBuf>,
}

/// Parsed `file=path[{ranges}]` directive: the file path plus an optional
/// list of 1-based inclusive line ranges to slice from the imported source.
type FileMeta = (String, Option<Vec<(usize, usize)>>);

impl Default for CodeImport {
  fn default() -> Self {
    Self::new()
  }
}

impl CodeImport {
  pub fn new() -> Self {
    Self { base_dir: None }
  }

  pub fn with_base_dir(p: impl Into<PathBuf>) -> Self {
    Self { base_dir: Some(p.into()) }
  }

  fn parse_file_meta(meta: &str) -> Option<FileMeta> {
    for part in meta.split_whitespace() {
      if let Some(rest) = part.strip_prefix("file=") {
        let raw = rest.trim_matches(|c| c == '"' || c == '\'');
        if let Some((path, range)) = raw.split_once('{') {
          let range = range.trim_end_matches('}');
          return Some((path.to_string(), Some(Self::parse_ranges(range))));
        }
        return Some((raw.to_string(), None));
      }
    }
    None
  }

  /// Discards malformed tokens silently.
  fn parse_ranges(spec: &str) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    for token in spec.split(',') {
      let token = token.trim();
      if let Some((a, b)) = token.split_once('-') {
        if let (Ok(a), Ok(b)) = (a.trim().parse::<usize>(), b.trim().parse::<usize>())
          && a >= 1
          && b >= a
        {
          out.push((a, b));
        }
      } else if let Ok(n) = token.parse::<usize>()
        && n >= 1
      {
        out.push((n, n));
      }
    }
    out
  }

  /// 1-based inclusive ranges; each picked line gets a trailing `\n`.
  fn slice_lines(src: &str, ranges: &[(usize, usize)]) -> String {
    let lines: Vec<&str> = src.lines().collect();
    let mut out = String::new();
    for (a, b) in ranges {
      let start = a.saturating_sub(1);
      let end = (*b).min(lines.len());
      for l in lines.iter().take(end).skip(start) {
        out.push_str(l);
        out.push('\n');
      }
    }
    out
  }
}

impl Transformer for CodeImport {
  fn name(&self) -> &str {
    "code-import"
  }

  fn transform(&self, doc: &mut Document, meta: &SourceMeta, engine: &mut duck_diagnostic::DiagnosticEngine<Code>) {
    let base_dir = self.base_dir.clone().or_else(|| match &meta.origin {
      Origin::File(p) => p.parent().map(|p| p.to_path_buf()),
      _ => None,
    });

    // Walk continues even on warning so absolute `file=` paths still resolve.
    if base_dir.is_none() {
      engine.emit(Diagnostic::new(
        Code::BaseDirNotFound,
        format!(
          "code-import: source has no on-disk parent (origin = {:?}); relative `file=` paths cannot be resolved",
          meta.origin
        ),
      ));
    }

    let mut v = Apply { base_dir, meta_path: meta.path.clone(), pending: Vec::new() };
    walk_root(&mut doc.children, &mut v);
    for d in v.pending.drain(..) {
      engine.emit(d);
    }
  }
}

struct Apply {
  base_dir: Option<PathBuf>,
  meta_path: Arc<str>,
  pending: Vec<Diagnostic<Code>>,
}

impl Visitor for Apply {
  fn visit_node(&mut self, node: &mut Node) -> NodeAction {
    if let Node::CodeBlock(cb) = node
      && let Some(meta) = cb.meta.as_deref()
      && let Some((file, ranges)) = CodeImport::parse_file_meta(meta)
    {
      if let Some(rs) = &ranges
        && rs.is_empty()
      {
        self.pending.push(Diagnostic::new(
          Code::InvalidLineRange,
          format!("code-import: line range in `{}` is empty / malformed", meta),
        ));
        return NodeAction::Keep;
      }

      let path = match &self.base_dir {
        Some(b) => b.join(&file),
        None => PathBuf::from(&file),
      };
      match std::fs::read_to_string(&path) {
        Ok(content) => {
          cb.value = match ranges {
            Some(rs) => CodeImport::slice_lines(&content, &rs),
            None => content,
          };
        },
        Err(e) => {
          self.pending.push(
            Diagnostic::new(Code::ImportFileNotFound, format!("code-import: cannot read {} ({})", path.display(), e))
              .with_label(Label::primary(cb.span.clone(), Some(format!("imported from {}", self.meta_path)))),
          );
        },
      }
    }
    NodeAction::Keep
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parse_file_meta_no_range() {
    assert_eq!(CodeImport::parse_file_meta("file=foo.rs"), Some(("foo.rs".into(), None)));
    assert_eq!(CodeImport::parse_file_meta("file=\"foo.rs\""), Some(("foo.rs".into(), None)));
  }

  #[test]
  fn parse_file_meta_with_range() {
    let (p, r) = CodeImport::parse_file_meta("file=foo.rs{1,3-5,8}").unwrap();
    assert_eq!(p, "foo.rs");
    assert_eq!(r, Some(vec![(1, 1), (3, 5), (8, 8)]));
  }

  #[test]
  fn slice_lines_picks_ranges() {
    let src = "a\nb\nc\nd\ne\n";
    let out = CodeImport::slice_lines(src, &[(2, 3)]);
    assert_eq!(out, "b\nc\n");
    let out = CodeImport::slice_lines(src, &[(1, 1), (4, 5)]);
    assert_eq!(out, "a\nd\ne\n");
  }
}
