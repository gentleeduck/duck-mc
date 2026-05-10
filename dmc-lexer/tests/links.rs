mod common;
use common::*;
use dmc_lexer::token::TokenKind;
use pretty_assertions::assert_eq;

#[test]
fn inline_link_emits_boundaries() {
  let kinds = lex_kinds("[text](url)\n");
  assert!(kinds.contains(&TokenKind::LinkOpen), "got {:?}", kinds);
  assert!(kinds.contains(&TokenKind::LinkClose));
  assert!(kinds.contains(&TokenKind::LinkTargetOpen));
  assert!(kinds.contains(&TokenKind::LinkTargetClose));
}

#[test]
fn image_marker() {
  let kinds = lex_kinds("![alt](url)\n");
  assert!(kinds.contains(&TokenKind::ImageMarker), "got {:?}", kinds);
}

#[test]
fn link_ref_definition() {
  let kinds = lex_kinds("[label]: https://example.com\n");
  assert!(kinds.contains(&TokenKind::LinkRefDef), "got {:?}", kinds);
}

#[test]
fn indented_link_ref_is_not_def() {
  // Column 1+ keeps `[` `]` as LinkOpen/LinkClose, not LinkRefDef.
  let kinds = lex_kinds("  [label]: https://x.com\n");
  assert!(!kinds.contains(&TokenKind::LinkRefDef), "got {:?}", kinds);
  assert!(kinds.contains(&TokenKind::LinkOpen));
  assert!(kinds.contains(&TokenKind::LinkClose));
}

#[test]
fn footnote_reference() {
  let kinds = lex_kinds("see[^1] here\n");
  assert!(kinds.contains(&TokenKind::FootnoteRefOpen), "got {:?}", kinds);
}

#[test]
fn footnote_definition() {
  let kinds = lex_kinds("[^1]: definition\n");
  assert!(kinds.contains(&TokenKind::FootnoteDefMarker), "got {:?}", kinds);
}

#[test]
fn empty_footnote_label_falls_through() {
  let kinds = lex_kinds("[^] not a footnote\n");
  assert!(!kinds.contains(&TokenKind::FootnoteRefOpen), "got {:?}", kinds);
}

#[test]
fn shortcut_link_emits_open_close() {
  let kinds = lex_kinds("[label]\n");
  let opens = kinds.iter().filter(|k| matches!(k, TokenKind::LinkOpen)).count();
  let closes = kinds.iter().filter(|k| matches!(k, TokenKind::LinkClose)).count();
  assert_eq!(opens, 1);
  assert_eq!(closes, 1);
}
