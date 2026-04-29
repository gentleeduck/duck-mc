use crate::pipeline::Transformer;
use crate::visit::{NodeAction, Visitor, walk_root};
use duck_diagnostic::{Diagnostic, Label};
use duck_md_diagnostic::Code;
use duck_md_diagnostic::metadata::{Origin, SourceMeta};
use duck_md_parser::ast::*;
use std::path::PathBuf;
use std::sync::Arc;

/// Replace `<ComponentSource path="…" />` with a `CodeBlock` carrying the
/// contents of the file at `path` (resolved against `base_dir`). The block's
/// `lang` is set from the file extension.
///
/// Path resolution mirrors `CodeImport`: explicit `base_dir` → mdx parent →
/// cwd (with [`Code::BaseDirNotFound`] warning).
#[derive(Default)]
pub struct ComponentSource {
  pub base_dir: Option<PathBuf>,
}

impl ComponentSource {
  pub fn with_base_dir(p: impl Into<PathBuf>) -> Self {
    Self { base_dir: Some(p.into()) }
  }

  fn attr_value(attrs: &[JsxAttr], name: &str) -> Option<String> {
    attrs.iter().find(|a| a.name == name).and_then(|a| match &a.value {
      JsxAttrValue::String(s) => Some(s.clone()),
      _ => None,
    })
  }
}

impl Transformer for ComponentSource {
  fn name(&self) -> &str {
    "component-source"
  }
  fn transform(
    &self,
    doc: &mut Document,
    meta: &SourceMeta,
    engine: &mut duck_diagnostic::DiagnosticEngine<Code>,
  ) {
    let base_dir = self.base_dir.clone().or_else(|| match &meta.origin {
      Origin::File(p) => p.parent().map(|p| p.to_path_buf()),
      _ => None,
    });

    if base_dir.is_none() && self.base_dir.is_none() {
      engine.emit(Diagnostic::new(
        Code::BaseDirNotFound,
        format!(
          "component-source: source has no on-disk parent (origin = {:?}); relative `path=` cannot be resolved",
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
    let (path, span) = match node {
      Node::JsxSelfClosing(j) if j.name == "ComponentSource" => {
        (ComponentSource::attr_value(&j.attrs, "path"), j.span.clone())
      },
      Node::JsxElement(j) if j.name == "ComponentSource" => {
        (ComponentSource::attr_value(&j.attrs, "path"), j.span.clone())
      },
      _ => return NodeAction::Keep,
    };
    let Some(rel) = path else {
      self.pending.push(
        Diagnostic::new(
          Code::MissingComponentAttr,
          "component-source: missing required `path` attribute".to_string(),
        )
        .with_label(Label::primary(span, Some("on this <ComponentSource>".into()))),
      );
      return NodeAction::Keep;
    };
    let abs = match &self.base_dir {
      Some(b) => b.join(&rel),
      None => PathBuf::from(&rel),
    };
    match std::fs::read_to_string(&abs) {
      Ok(content) => {
        let lang = abs.extension().and_then(|s| s.to_str()).map(String::from);
        *node = Node::CodeBlock(CodeBlock {
          lang,
          meta: Some(format!("title=\"{}\"", rel)),
          value: content,
          span,
        });
        NodeAction::KeepSkipChildren
      },
      Err(e) => {
        self.pending.push(
          Diagnostic::new(
            Code::ComponentSourceUnreadable,
            format!("component-source: cannot read {} ({})", abs.display(), e),
          )
          .with_label(Label::primary(span, Some(format!("from {}", self.meta_path)))),
        );
        NodeAction::Keep
      },
    }
  }
}
