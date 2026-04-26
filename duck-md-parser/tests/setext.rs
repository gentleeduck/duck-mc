mod common;
use common::*;
use duck_md_parser::ast::*;

#[test]
fn setext_h1() {
  let d = parse_doc("Title\n=====\n");
  let h = d.children.iter().find_map(|n| match n {
    Node::Heading(h) if h.level == 1 => Some(h),
    _ => None,
  });
  assert!(h.is_some(), "got {:?}", d.children);
}

#[test]
fn setext_h2() {
  let d = parse_doc("Title\n-----\n");
  let h = d.children.iter().find_map(|n| match n {
    Node::Heading(h) if h.level == 2 => Some(h),
    _ => None,
  });
  assert!(h.is_some(), "got {:?}", d.children);
}
