use crate::pipeline::Transformer;
use crate::visit::{walk_mut, VisitFlow, Visitor};
use duck_md_ast::*;
use std::path::PathBuf;

#[derive(Default)]
pub struct CodeImport {
  pub base_dir: Option<PathBuf>,
}

impl CodeImport {
  pub fn with_base_dir(p: impl Into<PathBuf>) -> Self {
    Self {
      base_dir: Some(p.into()),
    }
  }
}

impl Transformer for CodeImport {
  fn name(&self) -> &str {
    "code-import"
  }

  fn transform(&self, doc: &mut Document) {
    let mut v = Apply {
      base_dir: self.base_dir.clone(),
    };
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
      && let Some(file) = parse_file_meta(meta)
    {
      let path = match &self.base_dir {
        Some(b) => b.join(&file),
        None => PathBuf::from(&file),
      };
      if let Ok(content) = std::fs::read_to_string(&path) {
        cb.value = content;
      }
    }
    VisitFlow::Continue
  }
}

fn parse_file_meta(meta: &str) -> Option<String> {
  // accept `file=path` or `file="path"`
  for part in meta.split_whitespace() {
    if let Some(rest) = part.strip_prefix("file=") {
      let p = rest.trim_matches(|c| c == '"' || c == '\'');
      return Some(p.to_string());
    }
  }
  None
}
