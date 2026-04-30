mod common;
use common::*;
use dmc_lexer::token::TokenKind;

#[test]
fn strike_emits_token() {
  let kinds = lex_kinds("~~bye~~\n");
  assert!(kinds.iter().any(|k| matches!(k, TokenKind::Strike(2))), "got {:?}", kinds);
}

#[test]
fn single_tilde_is_text() {
  let kinds = lex_kinds("a~b\n");
  assert!(!kinds.iter().any(|k| matches!(k, TokenKind::Strike(_))), "got {:?}", kinds);
}
