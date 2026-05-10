mod common;
use common::*;
use dmc_lexer::token::TokenKind;

#[test]
fn strikethrough_emits_pair() {
  let kinds = lex_kinds("~~bye~~\n");
  let n = kinds.iter().filter(|k| matches!(k, TokenKind::Strikethrough)).count();
  assert_eq!(n, 2, "expected open + close, got {:?}", kinds);
}

#[test]
fn single_tilde_is_text() {
  let kinds = lex_kinds("a~b\n");
  assert!(!kinds.iter().any(|k| matches!(k, TokenKind::Strikethrough)), "got {:?}", kinds);
}

#[test]
fn triple_tilde_is_not_strike() {
  // `~~~` looks like a fence opener mid-paragraph; should stay text.
  let kinds = lex_kinds("a~~~b\n");
  assert!(!kinds.iter().any(|k| matches!(k, TokenKind::Strikethrough)), "got {:?}", kinds);
}

#[test]
fn strike_around_inline_code() {
  let kinds = lex_kinds("~~before `x` after~~\n");
  let strikes = kinds.iter().filter(|k| matches!(k, TokenKind::Strikethrough)).count();
  assert_eq!(strikes, 2);
  assert!(kinds.contains(&TokenKind::CodeInlineOpen(1)));
}
