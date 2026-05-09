//! Heading id assignment. See `transformers/assign-heading-ids.md` for
//! full docs.

use crate::pipeline::Transformer;
use dmc_diagnostic::Code;
use dmc_diagnostic::metadata::SourceMeta;
use dmc_parser::Slugger;
use dmc_parser::ast::*;

#[derive(Default, Debug)]
pub struct AssignHeadingIds;

impl AssignHeadingIds {
  pub fn new() -> Self {
    Self
  }
}

impl Transformer for AssignHeadingIds {
  fn name(&self) -> &str {
    "assign-heading-ids"
  }

  fn transform(
    &self,
    doc: &mut Document,
    _meta: &SourceMeta,
    _diag_engine: &mut duck_diagnostic::DiagnosticEngine<Code>,
  ) {
    let mut slugger = Slugger::new();
    walk(&mut doc.children, &mut slugger);
  }
}

fn walk(nodes: &mut [Node], slugger: &mut Slugger) {
  for node in nodes {
    if let Node::Heading(h) = node {
      if h.id.is_none() {
        let text = Heading::plain_text(&h.children);
        h.id = Some(slugger.slug(&text));
      }
      // Headings can't contain other headings; no need to recurse.
      continue;
    }
    if let Some(children) = Node::children_of_mut(node) {
      walk(children, slugger);
    }
  }
}
