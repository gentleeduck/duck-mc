use duck_md_parser::ast::*;

#[derive(Debug, Clone, Copy)]
pub enum VisitFlow {
  Continue,
  SkipChildren,
}

pub trait Visitor {
  fn visit_node(&mut self, _node: &mut Node) -> VisitFlow {
    VisitFlow::Continue
  }
}

pub fn walk_mut<V: Visitor>(node: &mut Node, v: &mut V) {
  let flow = v.visit_node(node);
  if let VisitFlow::SkipChildren = flow {
    return;
  }
  walk_children_mut(node, v);
}

fn walk_children_mut<V: Visitor>(node: &mut Node, v: &mut V) {
  match node {
    Node::Document(d) => {
      for c in &mut d.children {
        walk_mut(c, v);
      }
    },
    Node::Heading(h) => {
      for c in &mut h.children {
        walk_mut(c, v);
      }
    },
    Node::Paragraph(p) => {
      for c in &mut p.children {
        walk_mut(c, v);
      }
    },
    Node::Bold(i) | Node::Italic(i) | Node::Strikethrough(i) => {
      for c in &mut i.children {
        walk_mut(c, v);
      }
    },
    Node::Link(l) => {
      for c in &mut l.children {
        walk_mut(c, v);
      }
    },
    Node::List(l) => {
      for c in &mut l.children {
        walk_mut(c, v);
      }
    },
    Node::ListItem(li) => {
      for c in &mut li.children {
        walk_mut(c, v);
      }
    },
    Node::TaskListItem(t) => {
      for c in &mut t.children {
        walk_mut(c, v);
      }
    },
    Node::Blockquote(b) => {
      for c in &mut b.children {
        walk_mut(c, v);
      }
    },
    Node::Table(t) => {
      for r in &mut t.children {
        // TableRow has cells, each cell has children
        for cell in &mut r.cells {
          for c in &mut cell.children {
            walk_mut(c, v);
          }
        }
      }
    },
    Node::JsxElement(e) => {
      for c in &mut e.children {
        walk_mut(c, v);
      }
    },
    Node::JsxFragment(f) => {
      for c in &mut f.children {
        walk_mut(c, v);
      }
    },
    _ => {},
  }
}
