use dmc_parser::ast::*;
use dmc_transform::{BareUrlAutolink, Pipeline};

#[test]
fn rewrites_bare_url_in_paragraph() {
  let mut d = dmc_parser::parse("see https://example.com for info\n");
  Pipeline::new().add(BareUrlAutolink).run_silent(&mut d);
  let p = d
    .children
    .iter()
    .find_map(|n| match n {
      Node::Paragraph(p) => Some(p),
      _ => None,
    })
    .expect("paragraph");
  let has_link =
    p.children.iter().any(|n| matches!(n, Node::Link(l) if l.href == "https://example.com"));
  assert!(has_link, "got {:?}", p.children);
}

#[test]
fn does_not_rewrite_when_no_url() {
  let mut d = dmc_parser::parse("nothing here\n");
  Pipeline::new().add(BareUrlAutolink).run_silent(&mut d);
  let p = d
    .children
    .iter()
    .find_map(|n| match n {
      Node::Paragraph(p) => Some(p),
      _ => None,
    })
    .expect("paragraph");
  assert!(p.children.iter().all(|n| matches!(n, Node::Text(_))));
}
