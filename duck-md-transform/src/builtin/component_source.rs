use crate::pipeline::Transformer;
use crate::visit::{VisitFlow, Visitor, walk_mut};
use duck_md_parser::ast::*;
use std::path::PathBuf;

#[derive(Default)]
pub struct ComponentSource {
  pub base_dir: Option<PathBuf>,
}

impl ComponentSource {
  pub fn with_base_dir(p: impl Into<PathBuf>) -> Self {
    Self { base_dir: Some(p.into()) }
  }
}

impl Transformer for ComponentSource {
  fn name(&self) -> &str { "component-source" }
  fn transform(&self, doc: &mut Document) {
    let mut v = Apply { base_dir: self.base_dir.clone() };
    for c in &mut doc.children { walk_mut(c, &mut v); }
  }
}

struct Apply { base_dir: Option<PathBuf> }

impl Visitor for Apply {
  fn visit_node(&mut self, node: &mut Node) -> VisitFlow {
    let path = match node {
      Node::JsxSelfClosing(j) if j.name == "ComponentSource" => attr_value(&j.attrs, "path"),
      Node::JsxElement(j) if j.name == "ComponentSource" => attr_value(&j.attrs, "path"),
      _ => return VisitFlow::Continue,
    };
    let Some(rel) = path else { return VisitFlow::Continue };
    let abs = match &self.base_dir {
      Some(b) => b.join(&rel),
      None => PathBuf::from(&rel),
    };
    let Ok(content) = std::fs::read_to_string(&abs) else { return VisitFlow::Continue };
    let lang = abs.extension().and_then(|s| s.to_str()).map(String::from);
    *node = Node::CodeBlock(CodeBlock {
      lang,
      meta: Some(format!("title=\"{}\"", rel)),
      value: content,
      raw: None,
      commands: None,
      highlighted_html: None,
      span: default_span(),
    });
    VisitFlow::SkipChildren
  }
}

fn attr_value(attrs: &[JsxAttr], name: &str) -> Option<String> {
  for a in attrs {
    if a.name == name {
      return match &a.value {
        JsxAttrValue::String(s) => Some(s.clone()),
        _ => None,
      };
    }
  }
  None
}
