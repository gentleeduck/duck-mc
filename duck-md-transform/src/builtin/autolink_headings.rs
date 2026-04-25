use crate::pipeline::Transformer;
use crate::visit::{VisitFlow, Visitor, walk_mut};
use duck_md_parser::ast::*;

#[derive(Default)]
pub struct AutolinkHeadings {
  pub class_name: Option<String>,
  pub aria_label: Option<String>,
}

impl AutolinkHeadings {
  pub fn new() -> Self {
    Self {
      class_name: Some("subheading-anchor".to_string()),
      aria_label: Some("Link to section".to_string()),
    }
  }
}

impl Transformer for AutolinkHeadings {
  fn name(&self) -> &str {
    "autolink-headings"
  }

  fn transform(&self, doc: &mut Document) {
    let mut v = Apply { class_name: self.class_name.clone(), aria_label: self.aria_label.clone() };
    for c in &mut doc.children {
      walk_mut(c, &mut v);
    }
  }
}

struct Apply {
  #[allow(dead_code)]
  class_name: Option<String>,
  aria_label: Option<String>,
}

impl Visitor for Apply {
  fn visit_node(&mut self, node: &mut Node) -> VisitFlow {
    if let Node::Heading(h) = node {
      // Wrap heading children in a Link with href = "#<id>"
      // We model the autolink as a `Link` whose href is the heading anchor and
      // whose `title` carries the aria-label/class semantically.
      let original = std::mem::take(&mut h.children);
      // Skip if already wrapped in an autolink — detect by single child being a Link to "#<id>"
      let already =
        matches!(original.as_slice(), [Node::Link(l)] if l.href == format!("#{}", h.id));
      if already {
        h.children = original;
        return VisitFlow::SkipChildren;
      }
      let link = Node::Link(Link {
        href: format!("#{}", h.id),
        title: self.aria_label.clone(),
        children: original,
        span: default_span(),
      });
      h.children = vec![link];
      return VisitFlow::SkipChildren;
    }
    VisitFlow::Continue
  }
}
