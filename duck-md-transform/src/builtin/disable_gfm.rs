use crate::pipeline::Transformer;
use duck_md_parser::ast::*;

#[derive(Default)]
pub struct DisableGfm;

impl Transformer for DisableGfm {
  fn name(&self) -> &str { "disable-gfm" }
  fn transform(&self, doc: &mut Document) {
    rewrite(&mut doc.children);
  }
}

fn rewrite(nodes: &mut Vec<Node>) {
  for node in nodes.iter_mut() {
    match node {
      Node::Strikethrough(inner) => {
        rewrite(&mut inner.children);
        let mut buf = String::from("~~");
        flatten(&inner.children, &mut buf);
        buf.push_str("~~");
        *node = Node::Text(Text { value: buf, span: default_span() });
      }
      Node::Table(t) => {
        let mut buf = String::new();
        for row in &t.children {
          for (i, cell) in row.cells.iter().enumerate() {
            if i > 0 { buf.push_str(" | "); }
            flatten(&cell.children, &mut buf);
          }
          buf.push('\n');
        }
        *node = Node::Paragraph(Paragraph {
          children: vec![Node::Text(Text { value: buf, span: default_span() })],
          span: default_span(),
        });
      }
      Node::TaskListItem(it) => {
        rewrite(&mut it.children);
        let prefix = if it.checked { "[x] " } else { "[ ] " };
        let li = ListItem {
          children: it.children.clone(),
          span: default_span(),
        };
        let mut new_li = li;
        if let Some(Node::Paragraph(p)) = new_li.children.first_mut() {
          p.children.insert(0, Node::Text(Text { value: prefix.into(), span: default_span() }));
        } else {
          new_li.children.insert(0, Node::Text(Text { value: prefix.into(), span: default_span() }));
        }
        *node = Node::ListItem(new_li);
      }
      Node::Paragraph(p) => rewrite(&mut p.children),
      Node::Heading(h) => rewrite(&mut h.children),
      Node::Bold(i) | Node::Italic(i) => rewrite(&mut i.children),
      Node::List(l) => rewrite(&mut l.children),
      Node::ListItem(li) => rewrite(&mut li.children),
      Node::Blockquote(b) => rewrite(&mut b.children),
      Node::Link(l) => rewrite(&mut l.children),
      Node::JsxElement(j) => rewrite(&mut j.children),
      Node::JsxFragment(f) => rewrite(&mut f.children),
      _ => {}
    }
  }
}

fn flatten(nodes: &[Node], buf: &mut String) {
  for n in nodes {
    match n {
      Node::Text(t) => buf.push_str(&t.value),
      Node::InlineCode(c) => {
        buf.push('`');
        buf.push_str(&c.value);
        buf.push('`');
      }
      Node::Bold(i) => { buf.push_str("**"); flatten(&i.children, buf); buf.push_str("**"); }
      Node::Italic(i) => { buf.push('*'); flatten(&i.children, buf); buf.push('*'); }
      Node::Link(l) => {
        buf.push('[');
        flatten(&l.children, buf);
        buf.push_str("](");
        buf.push_str(&l.href);
        buf.push(')');
      }
      _ => {}
    }
  }
}
