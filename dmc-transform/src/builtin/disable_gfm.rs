use crate::pipeline::Transformer;
use dmc_diagnostic::{Code, metadata::SourceMeta};
use dmc_parser::ast::*;

/// Strip GFM-only constructs by serialising them back to plain markdown:
/// `~~strike~~` becomes literal text, tables flatten to pipe-delimited text,
/// task list items lose their checkbox state and become plain list items.
#[derive(Default)]
pub struct DisableGfm;

impl Transformer for DisableGfm {
  fn name(&self) -> &str {
    "disable-gfm"
  }
  fn transform(
    &self,
    doc: &mut Document,
    #[allow(unused_variables)] meta: &SourceMeta,
    #[allow(unused_variables)] engine: &mut duck_diagnostic::DiagnosticEngine<Code>,
  ) {
    Self::rewrite(&mut doc.children);
  }
}

impl DisableGfm {
  /// Walk a `children` Vec, rewriting any GFM-only node into a plain
  /// equivalent. Recurses into containers along the way.
  fn rewrite(nodes: &mut [Node]) {
    for node in nodes.iter_mut() {
      match node {
        Node::Strikethrough(inner) => {
          let span = inner.span.clone();
          Self::rewrite(&mut inner.children);
          let mut buf = String::from("~~");
          Self::flatten(&inner.children, &mut buf);
          buf.push_str("~~");
          *node = Node::Text(Text { value: buf, span });
        },
        Node::Table(t) => {
          let span = t.span.clone();
          let mut buf = String::new();
          for row in &t.children {
            for (i, cell) in row.cells.iter().enumerate() {
              if i > 0 {
                buf.push_str(" | ");
              }
              Self::flatten(&cell.children, &mut buf);
            }
            buf.push('\n');
          }
          *node = Node::Paragraph(Paragraph {
            children: vec![Node::Text(Text { value: buf, span: span.clone() })],
            span,
          });
        },
        Node::TaskListItem(it) => {
          let span = it.span.clone();
          Self::rewrite(&mut it.children);
          let prefix = if it.checked { "[x] " } else { "[ ] " };
          let mut new_li = ListItem { children: it.children.clone(), span: span.clone() };
          if let Some(Node::Paragraph(p)) = new_li.children.first_mut() {
            p.children.insert(0, Node::Text(Text { value: prefix.into(), span }));
          } else {
            new_li.children.insert(0, Node::Text(Text { value: prefix.into(), span }));
          }
          *node = Node::ListItem(new_li);
        },
        Node::Paragraph(p) => Self::rewrite(&mut p.children),
        Node::Heading(h) => Self::rewrite(&mut h.children),
        Node::Bold(i) | Node::Italic(i) => Self::rewrite(&mut i.children),
        Node::List(l) => Self::rewrite(&mut l.children),
        Node::ListItem(li) => Self::rewrite(&mut li.children),
        Node::Blockquote(b) => Self::rewrite(&mut b.children),
        Node::Link(l) => Self::rewrite(&mut l.children),
        Node::JsxElement(j) => Self::rewrite(&mut j.children),
        Node::JsxFragment(f) => Self::rewrite(&mut f.children),
        _ => {},
      }
    }
  }

  /// Flatten an inline subtree into a plain markdown-ish string. Used by
  /// `rewrite` when serialising a GFM container's contents into a Text node.
  fn flatten(nodes: &[Node], buf: &mut String) {
    for n in nodes {
      match n {
        Node::Text(t) => buf.push_str(&t.value),
        Node::InlineCode(c) => {
          buf.push('`');
          buf.push_str(&c.value);
          buf.push('`');
        },
        Node::Bold(i) => {
          buf.push_str("**");
          Self::flatten(&i.children, buf);
          buf.push_str("**");
        },
        Node::Italic(i) => {
          buf.push('*');
          Self::flatten(&i.children, buf);
          buf.push('*');
        },
        Node::Link(l) => {
          buf.push('[');
          Self::flatten(&l.children, buf);
          buf.push_str("](");
          buf.push_str(&l.href);
          buf.push(')');
        },
        _ => {},
      }
    }
  }
}
