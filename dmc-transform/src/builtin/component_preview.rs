use crate::pipeline::Transformer;
use crate::visit::{NodeAction, Visitor, walk_root};
use dmc_diagnostic::Code;
use dmc_diagnostic::metadata::SourceMeta;
use dmc_parser::ast::*;
use duck_diagnostic::{Diagnostic, Label};
use std::path::PathBuf;

/// Replace `<ComponentPreview name="X" />` with a `CodeBlock` carrying the
/// source of registry component `X`. `registry_index` is the JSON manifest;
/// `registry_root` is the directory referenced paths resolve against.
#[derive(Default)]
pub struct ComponentPreview {
  pub registry_index: Option<PathBuf>,
  pub registry_root: Option<PathBuf>,
}

impl ComponentPreview {
  /// Both paths required. `Default` leaves them `None` so the pass no-ops.
  pub fn new(registry_index: PathBuf, registry_root: PathBuf) -> Self {
    Self { registry_index: Some(registry_index), registry_root: Some(registry_root) }
  }

  /// Lookup by name. Index may be a JSON array of `{name, ...}` objects or
  /// an object keyed by name.
  fn lookup_entry<'a>(index: &'a serde_json::Value, name: &str) -> Option<&'a serde_json::Value> {
    if let Some(arr) = index.as_array() {
      arr.iter().find(|e| e.get("name").and_then(|v| v.as_str()) == Some(name))
    } else if let Some(obj) = index.as_object() {
      obj.get(name)
    } else {
      None
    }
  }

  fn attr_value(attrs: &[JsxAttr], name: &str) -> Option<String> {
    attrs.iter().find(|a| a.name == name).and_then(|a| match &a.value {
      JsxAttrValue::String(s) => Some(s.clone()),
      _ => None,
    })
  }
}

impl Transformer for ComponentPreview {
  fn name(&self) -> &str {
    "component-preview"
  }
  fn transform(
    &self,
    doc: &mut Document,
    _meta: &SourceMeta,
    diag_engine: &mut duck_diagnostic::DiagnosticEngine<Code>,
  ) {
    // Both paths required; missing either silently no-ops the pass.
    let Some(idx) = &self.registry_index else { return };
    let Some(root) = &self.registry_root else { return };
    let raw = match std::fs::read_to_string(idx) {
      Ok(r) => r,
      Err(e) => {
        diag_engine.emit(Diagnostic::new(
          Code::RegistryIndexUnreadable,
          format!("component-preview: cannot read registry index {} ({})", idx.display(), e),
        ));
        return;
      },
    };
    let index: serde_json::Value = match serde_json::from_str(&raw) {
      Ok(v) => v,
      Err(e) => {
        diag_engine.emit(Diagnostic::new(
          Code::RegistryIndexMalformed,
          format!("component-preview: registry index {} is not valid JSON ({})", idx.display(), e),
        ));
        return;
      },
    };
    let mut v = Apply { index, root: root.clone(), pending: Vec::new() };
    walk_root(&mut doc.children, &mut v);
    for d in v.pending.drain(..) {
      diag_engine.emit(d);
    }
  }
}

struct Apply {
  index: serde_json::Value,
  root: PathBuf,
  pending: Vec<Diagnostic<Code>>,
}

impl Visitor for Apply {
  fn visit_node(&mut self, node: &mut Node) -> NodeAction {
    let (name_opt, span) = match node {
      Node::JsxSelfClosing(j) if j.name == "ComponentPreview" => {
        (ComponentPreview::attr_value(&j.attrs, "name"), j.span.clone())
      },
      Node::JsxElement(j) if j.name == "ComponentPreview" => {
        (ComponentPreview::attr_value(&j.attrs, "name"), j.span.clone())
      },
      _ => return NodeAction::Keep,
    };
    let Some(name) = name_opt else {
      self.pending.push(
        Diagnostic::new(Code::MissingComponentAttr, "component-preview: missing required `name` attribute".to_string())
          .with_label(Label::primary(span, Some("on this <ComponentPreview>".into()))),
      );
      return NodeAction::Keep;
    };
    let Some(entry) = ComponentPreview::lookup_entry(&self.index, &name) else {
      self.pending.push(
        Diagnostic::new(
          Code::RegistryEntryNotFound,
          format!("component-preview: registry has no entry for `{}`", name),
        )
        .with_label(Label::primary(span, Some("not found".into()))),
      );
      return NodeAction::Keep;
    };
    let files = entry.get("files").and_then(|v| v.as_array());
    let Some(files) = files else {
      self.pending.push(Diagnostic::new(
        Code::RegistryEntryNotFound,
        format!("component-preview: entry `{}` has no `files` array", name),
      ));
      return NodeAction::Keep;
    };
    let Some(first) = files.first() else {
      self.pending.push(Diagnostic::new(
        Code::RegistryEntryNotFound,
        format!("component-preview: entry `{}` has empty `files` array", name),
      ));
      return NodeAction::Keep;
    };
    let path = first.get("path").and_then(|v| v.as_str()).unwrap_or("");
    let abs = self.root.join(path);
    match std::fs::read_to_string(&abs) {
      Ok(content) => {
        let lang = abs.extension().and_then(|s| s.to_str()).map(String::from);
        *node = Node::CodeBlock(CodeBlock { lang, meta: Some(format!("title=\"{name}\"")), value: content, span });
        NodeAction::KeepSkipChildren
      },
      Err(e) => {
        self.pending.push(
          Diagnostic::new(
            Code::RegistrySourceUnreadable,
            format!("component-preview: cannot read {} ({})", abs.display(), e),
          )
          .with_label(Label::primary(span, Some(format!("for `{}`", name)))),
        );
        NodeAction::Keep
      },
    }
  }
}
