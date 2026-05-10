mod common;
use common::*;
use dmc_lexer::token::{OrderedSep, TokenKind};

#[test]
fn thematic_break_dashes() {
  let kinds = lex_kinds("---\n");
  assert!(kinds.contains(&TokenKind::ThematicBreak), "got {:?}", kinds);
}

#[test]
fn thematic_break_stars() {
  let kinds = lex_kinds("***\n");
  assert!(kinds.contains(&TokenKind::ThematicBreak));
}

#[test]
fn thematic_break_underscores() {
  let kinds = lex_kinds("___\n");
  assert!(kinds.contains(&TokenKind::ThematicBreak));
}

#[test]
fn thematic_break_with_spaces() {
  let kinds = lex_kinds("- - -\n");
  assert!(kinds.contains(&TokenKind::ThematicBreak));
}

#[test]
fn two_dashes_is_not_thematic_break() {
  let kinds = lex_kinds("--\n");
  assert!(!kinds.contains(&TokenKind::ThematicBreak), "got {:?}", kinds);
}

#[test]
fn block_quote_marker() {
  let kinds = lex_kinds("> hi\n");
  assert!(kinds.contains(&TokenKind::BlockQuoteMarker), "got {:?}", kinds);
}

#[test]
fn nested_block_quote() {
  let kinds = lex_kinds("> > deep\n");
  let n = kinds.iter().filter(|k| matches!(k, TokenKind::BlockQuoteMarker)).count();
  assert_eq!(n, 2, "got {:?}", kinds);
}

#[test]
fn unordered_list_dash() {
  let kinds = lex_kinds("- item\n");
  assert!(kinds.contains(&TokenKind::UnorderedListMarker), "got {:?}", kinds);
}

#[test]
fn unordered_list_plus() {
  let kinds = lex_kinds("+ item\n");
  assert!(kinds.contains(&TokenKind::UnorderedListMarker));
}

#[test]
fn unordered_list_star() {
  let kinds = lex_kinds("* item\n");
  assert!(kinds.contains(&TokenKind::UnorderedListMarker));
}

#[test]
fn dash_no_space_is_text() {
  let kinds = lex_kinds("-item\n");
  assert!(!kinds.contains(&TokenKind::UnorderedListMarker), "got {:?}", kinds);
}

#[test]
fn ordered_list_period() {
  let kinds = lex_kinds("1. one\n");
  assert!(kinds.contains(&TokenKind::OrderedListMarker(OrderedSep::Period)), "got {:?}", kinds);
}

#[test]
fn ordered_list_paren() {
  let kinds = lex_kinds("1) one\n");
  assert!(kinds.contains(&TokenKind::OrderedListMarker(OrderedSep::Paren)));
}

#[test]
fn version_string_not_ordered_list() {
  // 0.4.3 -- the `0.` would match a marker, but `4` after is not space.
  let kinds = lex_kinds("0.4.3\n");
  assert!(!kinds.iter().any(|k| matches!(k, TokenKind::OrderedListMarker(_))), "got {:?}", kinds);
}

#[test]
fn task_marker_unchecked() {
  let kinds = lex_kinds("- [ ] todo\n");
  assert!(kinds.contains(&TokenKind::TaskMarker(false)), "got {:?}", kinds);
}

#[test]
fn task_marker_checked() {
  let kinds = lex_kinds("- [x] done\n");
  assert!(kinds.contains(&TokenKind::TaskMarker(true)));
}
