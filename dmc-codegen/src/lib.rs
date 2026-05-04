//! Codegen layer: turn a parsed `Document` into renderable output.
//!
//! Two emitters live here:
//! - [`HtmlEmitter`] - static HTML (SSR / SSG output).
//! - [`MdxBodyEmitter`] - JS body for MDX runtime React rendering.
//!
//! Both implement [`NodeSink`]. A single [`Walker`] does one pre-order
//! DFS, fanning each node to every active sink — no N tree traversals.
mod escape;
pub mod highlight;
pub mod html;
pub mod mdx;
use dmc_parser::ast::{Document, Node};
pub use html::{HtmlEmitter, render_html};
pub use mdx::{MdxBodyEmitter, render_mdx_body};

/// Callback pair invoked by [`Walker`] at every node.
pub trait NodeSink {
  fn enter(&mut self, node: &Node, ctx: &WalkCtx);
  fn leave(&mut self, _node: &Node, _ctx: &WalkCtx) {}
}

/// Position info handed to every sink callback. Read-only.
pub struct WalkCtx<'a> {
  /// Ancestor count above this node. Top-level children = 0.
  pub depth: usize,
  /// Index among parent's children. 0-based.
  pub index: usize,
  /// `None` when visiting a top-level child of the document.
  pub parent: Option<&'a Node>,
}

impl<'a> WalkCtx<'a> {
  pub fn root() -> Self {
    Self { depth: 0, index: 0, parent: None }
  }
  pub fn child(&self, parent: &'a Node, index: usize) -> Self {
    Self { depth: self.depth + 1, index, parent: Some(parent) }
  }
}

/// Pre-order DFS over `doc.children`. At every node, every sink's
/// `enter` fires (in slice order); the walker then recurses into the
/// node's children; finally every sink's `leave` fires (in reverse
/// slice order, LIFO).
///
/// `Document` itself is not surfaced as a `Node::Document` event; the
/// walker iterates `doc.children` directly. Sinks needing a document
/// boundary subscribe to `Frontmatter` or to their first node.
pub struct Walker<'a> {
  doc: &'a Document,
}

impl<'a> Walker<'a> {
  pub fn new(doc: &'a Document) -> Self {
    Self { doc }
  }

  /// Drive the walk. `enter` fires slice-order, `leave` fires LIFO.
  pub fn walk(self, sinks: &mut [&mut dyn NodeSink]) {
    for (i, child) in self.doc.children.iter().enumerate() {
      Self::walk_node(child, &WalkCtx { depth: 0, index: i, parent: None }, sinks);
    }
  }

  fn walk_node(node: &'a Node, ctx: &WalkCtx<'a>, sinks: &mut [&mut dyn NodeSink]) {
    for sink in sinks.iter_mut() {
      sink.enter(node, ctx);
    }
    match node {
      // Table rows/cells aren't `Node`s — handled inline.
      Node::Table(t) => {
        for row in &t.children {
          for cell in &row.cells {
            for (i, kid) in cell.children.iter().enumerate() {
              Self::walk_node(kid, &ctx.child(node, i), sinks);
            }
          }
        }
      },
      _ => {
        for (i, kid) in Node::children_of(node).iter().enumerate() {
          Self::walk_node(kid, &ctx.child(node, i), sinks);
        }
      },
    }
    for sink in sinks.iter_mut().rev() {
      sink.leave(node, ctx);
    }
  }
}
