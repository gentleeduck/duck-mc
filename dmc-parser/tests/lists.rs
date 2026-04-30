mod common;
use common::*;
use dmc_parser::ast::*;
use pretty_assertions::assert_eq;

#[test]
fn unordered_list_three_items() {
  let src = "- one\n- two\n- three\n";
  let d = parse_doc(src);
  let l = d
    .children
    .iter()
    .find_map(|n| match n {
      Node::List(l) => Some(l),
      _ => None,
    })
    .expect("list");
  assert!(!l.ordered);
  assert_eq!(l.children.len(), 3);
}

#[test]
fn ordered_list_with_start() {
  let src = "3. three\n4. four\n";
  let d = parse_doc(src);
  let l = d
    .children
    .iter()
    .find_map(|n| match n {
      Node::List(l) => Some(l),
      _ => None,
    })
    .expect("list");
  assert!(l.ordered);
  assert_eq!(l.start, Some(3));
  assert_eq!(l.children.len(), 2);
}

#[test]
fn list_followed_by_heading() {
  let src = "- item\n# heading\n";
  let d = parse_doc(src);
  let has_list = d.children.iter().any(|n| matches!(n, Node::List(_)));
  let has_heading = d.children.iter().any(|n| matches!(n, Node::Heading(_)));
  assert!(has_list);
  assert!(has_heading);
}
