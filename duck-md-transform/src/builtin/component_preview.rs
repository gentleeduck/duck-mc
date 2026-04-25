use crate::pipeline::Transformer;
use crate::visit::{VisitFlow, Visitor, walk_mut};
use duck_md_parser::ast::*;
use std::path::PathBuf;

#[derive(Default)]
pub struct ComponentPreview {
  pub registry_index: Option<PathBuf>,
  pub registry_root: Option<PathBuf>,
}

impl ComponentPreview {
  pub fn new(registry_index: PathBuf, registry_root: PathBuf) -> Self {
    Self {
      registry_index: Some(registry_index),
      registry_root: Some(registry_root),
    }
  }
}

impl Transformer for ComponentPreview {
  fn name(&self) -> &str { "component-preview" }
  fn transform(&self, doc: &mut Document) {
    let Some(idx) = &self.registry_index else { return };
    let Some(root) = &self.registry_root else { return };
    let Ok(raw) = std::fs::read_to_string(idx) else { return };
    let Ok(index): Result<serde_json::Value, _> = serde_json::from_str(&raw) else { return };
    let mut v = Apply { index, root: root.clone() };
    for c in &mut doc.children { walk_mut(c, &mut v); }
  }
}

struct Apply {
  index: serde_json::Value,
  root: PathBuf,
}

impl Visitor for Apply {
  fn visit_node(&mut self, node: &mut Node) -> VisitFlow {
    let name = match node {
      Node::JsxSelfClosing(j) if j.name == "ComponentPreview" => attr_value(&j.attrs, "name"),
      Node::JsxElement(j) if j.name == "ComponentPreview" => attr_value(&j.attrs, "name"),
      _ => return VisitFlow::Continue,
    };
    let Some(name) = name else { return VisitFlow::Continue };
    let Some(entry) = lookup_entry(&self.index, &name) else { return VisitFlow::Continue };
    let files = entry.get("files").and_then(|v| v.as_array());
    let Some(files) = files else { return VisitFlow::Continue };
    let Some(first) = files.first() else { return VisitFlow::Continue };
    let path = first.get("path").and_then(|v| v.as_str()).unwrap_or("");
    let abs = self.root.join(path);
    let Ok(content) = std::fs::read_to_string(&abs) else { return VisitFlow::Continue };
    let lang = abs.extension().and_then(|s| s.to_str()).map(String::from);
    *node = Node::CodeBlock(CodeBlock {
      lang,
      meta: Some(format!("title=\"{name}\"")),
      value: content,
      raw: None,
      commands: None,
      highlighted_html: None,
      span: default_span(),
    });
    VisitFlow::SkipChildren
  }
}

fn lookup_entry<'a>(index: &'a serde_json::Value, name: &str) -> Option<&'a serde_json::Value> {
  if let Some(arr) = index.as_array() {
    for entry in arr {
      if entry.get("name").and_then(|v| v.as_str()) == Some(name) {
        return Some(entry);
      }
    }
    None
  } else if let Some(obj) = index.as_object() {
    obj.get(name)
  } else {
    None
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
