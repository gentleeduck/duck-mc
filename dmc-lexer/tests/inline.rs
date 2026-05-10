mod common;
use common::*;
use dmc_lexer::token::{EmphasisChar, TokenKind};

#[test]
fn emphasis_one_asterisk() {
  let kinds = lex_kinds("*italic*\n");
  let n = kinds.iter().filter(|k| matches!(k, TokenKind::Emphasis(EmphasisChar::Asterisk, 1))).count();
  assert_eq!(n, 2, "got {:?}", kinds);
}

#[test]
fn emphasis_two_asterisks() {
  let kinds = lex_kinds("**bold**\n");
  let n = kinds.iter().filter(|k| matches!(k, TokenKind::Emphasis(EmphasisChar::Asterisk, 2))).count();
  assert_eq!(n, 2);
}

#[test]
fn emphasis_three_asterisks() {
  let kinds = lex_kinds("***both***\n");
  let n = kinds.iter().filter(|k| matches!(k, TokenKind::Emphasis(EmphasisChar::Asterisk, 3))).count();
  assert_eq!(n, 2);
}

#[test]
fn emphasis_underscore() {
  let kinds = lex_kinds("_italic_\n");
  let n = kinds.iter().filter(|k| matches!(k, TokenKind::Emphasis(EmphasisChar::Underscore, 1))).count();
  assert_eq!(n, 2);
}

#[test]
fn inline_code_basic() {
  let kinds = lex_kinds("hi `code` end\n");
  assert!(kinds.contains(&TokenKind::CodeInlineOpen(1)), "got {:?}", kinds);
  assert!(kinds.contains(&TokenKind::CodeInlineClose(1)));
}

#[test]
fn inline_code_double_backtick() {
  let kinds = lex_kinds("a `` `back` `` b\n");
  assert!(kinds.contains(&TokenKind::CodeInlineOpen(2)));
  assert!(kinds.contains(&TokenKind::CodeInlineClose(2)));
}

#[test]
fn inline_code_unmatched_falls_back() {
  let kinds = lex_kinds("no `closer here\n");
  assert!(!kinds.contains(&TokenKind::CodeInlineOpen(1)), "got {:?}", kinds);
}

#[test]
fn fenced_code_backtick() {
  use dmc_lexer::token::FenceChar;
  let src = "```rust\nfn main() {}\n```\n";
  let kinds = lex_kinds(src);
  assert!(kinds.contains(&TokenKind::CodeFenceOpen(FenceChar::Backtick, 3)), "got {:?}", kinds);
  assert!(kinds.contains(&TokenKind::CodeFenceInfo));
  assert!(kinds.contains(&TokenKind::CodeFenceContent));
  assert!(kinds.contains(&TokenKind::CodeFenceClose(FenceChar::Backtick, 3)));
}

#[test]
fn entity_named() {
  let kinds = lex_kinds("a &amp; b\n");
  assert!(kinds.contains(&TokenKind::EntityRef), "got {:?}", kinds);
}

#[test]
fn entity_decimal() {
  let kinds = lex_kinds("&#42;\n");
  assert!(kinds.contains(&TokenKind::EntityRef));
}

#[test]
fn entity_hex() {
  let kinds = lex_kinds("&#x2A;\n");
  assert!(kinds.contains(&TokenKind::EntityRef));
}

#[test]
fn bare_ampersand_stays_text() {
  let kinds = lex_kinds("a & b\n");
  assert!(!kinds.contains(&TokenKind::EntityRef), "got {:?}", kinds);
}

#[test]
fn missing_semicolon_not_entity() {
  let kinds = lex_kinds("&amp not entity\n");
  assert!(!kinds.contains(&TokenKind::EntityRef), "got {:?}", kinds);
}
