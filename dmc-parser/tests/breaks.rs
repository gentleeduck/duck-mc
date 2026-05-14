mod common;
use common::*;
use dmc_parser::ast::*;

#[test]
fn thematic_break_emits_node() {
  let d = parse_doc("---\n");
  // `---` at file start tries frontmatter; with no closing `---` it falls
  // back to ThematicBreak.
  let has_hr = d.children.iter().any(|n| matches!(n, Node::HorizontalRule(_)));
  assert!(has_hr, "got {:?}", d.children);
}

#[test]
fn thematic_break_via_stars() {
  let d = parse_doc("***\n");
  let has_hr = d.children.iter().any(|n| matches!(n, Node::HorizontalRule(_)));
  assert!(has_hr, "got {:?}", d.children);
}

#[test]
fn thematic_break_via_underscores() {
  let d = parse_doc("___\n");
  let has_hr = d.children.iter().any(|n| matches!(n, Node::HorizontalRule(_)));
  assert!(has_hr, "got {:?}", d.children);
}

#[test]
fn blockquote_single_line() {
  let d = parse_doc("> quoted text\n");
  let bq = d
    .children
    .iter()
    .find_map(|n| match n {
      Node::Blockquote(b) => Some(b),
      _ => None,
    })
    .expect("bq");
  assert!(!bq.children.is_empty());
}

#[test]
fn blockquote_multi_line_collapses() {
  // CM: consecutive `>` lines join into one paragraph, space-separated.
  let d = parse_doc("> one\n> two\n");
  let bq = d
    .children
    .iter()
    .find_map(|n| match n {
      Node::Blockquote(b) => Some(b),
      _ => None,
    })
    .expect("bq");
  assert_eq!(bq.children.len(), 1, "expected 1 paragraph, got {}", bq.children.len());
  let para = match &bq.children[0] {
    Node::Paragraph(p) => p,
    _ => panic!("expected paragraph"),
  };
  let flat: String = para
    .children
    .iter()
    .filter_map(|n| match n {
      Node::Text(t) => Some(t.value.as_str()),
      _ => None,
    })
    .collect();
  assert!(flat.contains("one"));
  assert!(flat.contains("two"));
}
