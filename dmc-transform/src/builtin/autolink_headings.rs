//! Heading anchor links. See `transformers/autolink-headings.md` for
//! full docs.

use crate::pipeline::Transformer;
use crate::visit::{NodeAction, Visitor, walk_root};
use dmc_diagnostic::Code;
use dmc_diagnostic::metadata::SourceMeta;
use dmc_parser::ast::*;

/// Anchor injector for headings.
#[derive(Default, Debug)]
pub struct AutolinkHeadings {
  /// `aria-label` to put on the anchor. Defaults to `"Link to section"`.
  pub aria_label: Option<String>,
  /// `className` to put on the anchor. Defaults to `"subheading-anchor"`.
  pub class_name: Option<String>,
  /// `className` for the inner `<span>` icon. Defaults to `"icon icon-link"`.
  pub icon_class_name: Option<String>,
}

impl AutolinkHeadings {
  /// Default config matches velite's `rehype-autolink-headings` invocation.
  pub fn new() -> Self {
    Self::default()
  }

  fn aria(&self) -> &str {
    self.aria_label.as_deref().unwrap_or("Link to section")
  }
  fn class(&self) -> &str {
    self.class_name.as_deref().unwrap_or("subheading-anchor")
  }
  fn icon_class(&self) -> &str {
    self.icon_class_name.as_deref().unwrap_or("icon icon-link")
  }
}

impl Transformer for AutolinkHeadings {
  fn name(&self) -> &str {
    "autolink-headings"
  }

  fn transform(
    &self,
    doc: &mut Document,
    _meta: &SourceMeta,
    _diag_engine: &mut duck_diagnostic::DiagnosticEngine<Code>,
  ) {
    let mut v = Apply {
      aria_label: self.aria().to_string(),
      class_name: self.class().to_string(),
      icon_class_name: self.icon_class().to_string(),
    };
    walk_root(&mut doc.children, &mut v);
  }
}

struct Apply {
  aria_label: String,
  class_name: String,
  icon_class_name: String,
}

impl Visitor for Apply {
  fn visit_node(&mut self, node: &mut Node) -> NodeAction {
    if let Node::Heading(h) = node {
      let id = h.slug();
      let span = h.span.clone();
      // Idempotency: skip if first child is already an `<a>` JsxElement
      // pointing at this heading's anchor.
      let already_prepended = matches!(
        h.children.first(),
        Some(Node::JsxElement(e))
          if e.name == "a"
            && e.attrs.iter().any(|a| a.name == "href" && matches!(&a.value, JsxAttrValue::String(s) if s == &format!("#{}", id)))
      );
      if already_prepended {
        return NodeAction::KeepSkipChildren;
      }
      let anchor = build_anchor(&id, &self.aria_label, &self.class_name, &self.icon_class_name, span);
      h.children.insert(0, anchor);
      return NodeAction::KeepSkipChildren;
    }
    NodeAction::Keep
  }
}

/// Build the `<a aria-label class href><span class /></a>` tree.
fn build_anchor(id: &str, aria: &str, class: &str, icon_class: &str, span: duck_diagnostic::Span) -> Node {
  let attr = |name: &str, value: &str| JsxAttr {
    name: name.into(),
    value: JsxAttrValue::String(value.into()),
    span: span.clone(),
  };
  let icon = Node::JsxSelfClosing(JsxSelfClosing {
    name: "span".into(),
    attrs: vec![attr("className", icon_class)],
    span: span.clone(),
  });
  Node::JsxElement(JsxElement {
    name: "a".into(),
    attrs: vec![attr("aria-label", aria), attr("className", class), attr("href", &format!("#{}", id))],
    children: vec![icon],
    span,
  })
}
