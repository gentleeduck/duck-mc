mod common;
use common::*;
use dmc_parser::ast::*;

#[test]
fn thematic_break_emits_node() {
  let d = parse_doc("---\n");
  // first child may be HorizontalRule or might be a Paragraph if first-line --- is consumed by lex_frontmatter
  // For `---` not at column 0 of file start, lexer emits ThematicBreak — but `---` IS at start.
  // It will try to be frontmatter; without closing --- it falls back to ThematicBreak (per lex_frontmatter logic).
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
  // its first child is a Paragraph; that paragraph contains text
  assert!(!bq.children.is_empty());
}

#[test]
fn blockquote_multi_line_collapses() {
  let d = parse_doc("> one\n> two\n");
  let bq = d
    .children
    .iter()
    .find_map(|n| match n {
      Node::Blockquote(b) => Some(b),
      _ => None,
    })
    .expect("bq");
  assert!(bq.children.len() >= 2, "got {} paragraphs", bq.children.len());
}
