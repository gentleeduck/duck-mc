use dmc_parser::ast::*;

/// Result of visiting one node. Walker consumes this when iterating the
/// parent's children Vec.
pub enum NodeAction {
  /// Walker recurses into this node's children.
  Keep,
  /// Node stays in place but its children are not visited.
  KeepSkipChildren,
  /// Splice these nodes into the parent's `children` at the current index.
  /// Replacements are NOT re-visited (prevents infinite loops on
  /// transformers that produce nodes matching their own pattern).
  Replace(Vec<Node>),
  /// Drop this node from the parent's children.
  Remove,
}

/// Implemented by transform passes to react to each node. The default impl
/// keeps every node and recurses, so override only the variants you care about.
pub trait Visitor {
  fn visit_node(&mut self, _node: &mut Node) -> NodeAction {
    NodeAction::Keep
  }
}

/// Recurse into the per-variant inner children of `parent`. Leaf variants
/// (Text, InlineCode, CodeBlock, Image, JsxExpression, ...) are no-ops.
pub fn walk_children_mut<V: Visitor>(parent: &mut Node, v: &mut V) {
  if let Node::Table(t) = parent {
    for row in &mut t.children {
      for cell in &mut row.cells {
        walk_root(&mut cell.children, v);
      }
    }
    return;
  }

  if let Some(kids) = Node::children_of_mut(parent) {
    walk_root(kids, v);
  }
}

/// Drive the visitor over a `Vec<Node>`, honoring every `NodeAction` variant.
/// Replacements aren't re-visited but are descended into on a later pass if
/// the visitor returns `Keep`.
pub fn walk_root<V: Visitor>(children: &mut Vec<Node>, v: &mut V) {
  let mut i = 0;
  while i < children.len() {
    match v.visit_node(&mut children[i]) {
      NodeAction::Keep => {
        walk_children_mut(&mut children[i], v);
        i += 1;
      },
      NodeAction::KeepSkipChildren => {
        i += 1;
      },
      NodeAction::Replace(new) => {
        let n = new.len();
        children.splice(i..=i, new);
        i += n;
      },
      NodeAction::Remove => {
        children.remove(i);
      },
    }
  }
}
