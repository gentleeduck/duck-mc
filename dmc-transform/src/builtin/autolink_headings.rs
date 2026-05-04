use crate::pipeline::Transformer;
use crate::visit::{NodeAction, Visitor, walk_root};
use dmc_diagnostic::Code;
use dmc_diagnostic::metadata::SourceMeta;
use dmc_parser::ast::*;

/// Wrap each `Heading`'s children in a `Link` to its own `#id` anchor.
/// `aria_label` flows through as the link's `title`.
#[derive(Default)]
pub struct AutolinkHeadings {
  pub aria_label: Option<String>,
}

impl AutolinkHeadings {
  /// Construct with the conventional shadcn-style aria label. Use
  /// `Default::default()` for an unset label.
  pub fn new() -> Self {
    Self { aria_label: Some("Link to section".to_string()) }
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
    let mut v = Apply { aria_label: self.aria_label.clone() };
    walk_root(&mut doc.children, &mut v);
  }
}

struct Apply {
  aria_label: Option<String>,
}

impl Visitor for Apply {
  fn visit_node(&mut self, node: &mut Node) -> NodeAction {
    if let Node::Heading(h) = node {
      let slug = h.slug();
      let span = h.span.clone();
      let original = std::mem::take(&mut h.children);
      let already = matches!(original.as_slice(), [Node::Link(l)] if l.href == format!("#{}", slug));
      if already {
        h.children = original;
        return NodeAction::KeepSkipChildren;
      }
      let link =
        Node::Link(Link { href: format!("#{}", slug), title: self.aria_label.clone(), children: original, span });
      h.children = vec![link];
      return NodeAction::KeepSkipChildren;
    }
    NodeAction::Keep
  }
}
