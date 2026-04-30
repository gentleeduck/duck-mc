use dmc_parser::ast::*;

/// Result of visiting one node. Walker consumes this when iterating the
/// parent's children Vec.
pub enum NodeAction {
  /// Node stays in place. Walker recurses into its children (default).
  Keep,
  /// Node stays in place but children are not visited.
  KeepSkipChildren,
  /// Replace this node with zero or more new nodes. Walker splices them
  /// into the parent's `children` Vec at the current index. Replacements
  /// are NOT re-visited (prevents infinite loops on transformers that
  /// produce nodes matching their own pattern).
  Replace(Vec<Node>),
  /// Drop this node from the parent's children Vec.
  Remove,
}

/// Implemented by transform passes to react to each node in the AST. The
/// default impl keeps every node and recurses into all children, making it
/// safe to override only the variants a transformer cares about.
pub trait Visitor {
  fn visit_node(&mut self, _node: &mut Node) -> NodeAction {
    NodeAction::Keep
  }
}

/// Top-level entry. Visit `node` then recurse into its children unless the
/// visitor returned `KeepSkipChildren`. `Replace` / `Remove` returned at
/// this level are ignored — they need a parent's children Vec to act on.
/// Use this when you have a single node with no parent context (rare); for
/// document-wide walks call `walk_children_mut` on the `Document` directly.
pub fn walk_mut<V: Visitor>(node: &mut Node, v: &mut V) {
  match v.visit_node(node) {
    NodeAction::Keep => walk_children_mut(node, v),
    NodeAction::KeepSkipChildren => {},
    NodeAction::Replace(_) | NodeAction::Remove => {},
  }
}

/// Recurse into the per-variant inner children of `parent`. Variant-aware
/// dispatch: each container variant walks its `children` Vec; leaf variants
/// (Text, InlineCode, CodeBlock, Image, JsxExpression, etc.) are no-ops.
/// `Replace` / `Remove` returned by the visitor splice / drop entries in
/// place.
pub fn walk_children_mut<V: Visitor>(parent: &mut Node, v: &mut V) {
  match parent {
    Node::Document(d) => walk_root(&mut d.children, v),
    Node::Heading(h) => walk_root(&mut h.children, v),
    Node::Paragraph(p) => walk_root(&mut p.children, v),
    Node::Bold(i) | Node::Italic(i) | Node::Strikethrough(i) => walk_root(&mut i.children, v),
    Node::Link(l) => walk_root(&mut l.children, v),
    Node::List(l) => walk_root(&mut l.children, v),
    Node::ListItem(li) => walk_root(&mut li.children, v),
    Node::TaskListItem(t) => walk_root(&mut t.children, v),
    Node::Blockquote(b) => walk_root(&mut b.children, v),
    Node::JsxElement(e) => walk_root(&mut e.children, v),
    Node::JsxFragment(f) => walk_root(&mut f.children, v),
    Node::Table(t) => {
      // TableRow / TableCell aren't `Node` variants, so we descend
      // manually and run the per-cell loop on `cell.children`.
      for row in &mut t.children {
        for cell in &mut row.cells {
          walk_root(&mut cell.children, v);
        }
      }
    },
    _ => {},
  }
}

/// Drive the visitor over a `Vec<Node>`, honoring every `NodeAction` variant.
/// Replacements aren't re-visited (no infinite loop on self-producing
/// transformers); they ARE descended into via `walk_children_mut` only if
/// the visitor returns `Keep` for them on a future pass.
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
