use duck_md_parser::ast::*;

mod common;
use common::parse_doc;

fn first_paragraph_children(doc: &Document) -> &[Node] {
  for c in &doc.children {
    if let Node::Paragraph(p) = c {
      return &p.children;
    }
  }
  &[]
}

#[test]
fn parses_angle_autolink_to_link_node() {
  let doc = parse_doc("see <https://rust-lang.org> end");
  let kids = first_paragraph_children(&doc);
  let link = kids
    .iter()
    .find_map(|n| match n {
      Node::Link(l) => Some(l),
      _ => None,
    })
    .expect("expected a Link node");
  assert_eq!(link.href, "https://rust-lang.org");
  assert_eq!(link.children.len(), 1);
  if let Node::Text(t) = &link.children[0] {
    assert_eq!(t.value, "https://rust-lang.org");
  } else {
    panic!("link child should be Text");
  }
}
