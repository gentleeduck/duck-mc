mod common;
use common::*;
use dmc_lexer::token::TokenKind;
use pretty_assertions::assert_eq;

#[test]
fn lt_then_digit_is_text() {
  let kinds = lex_kinds("5 < 10\n");
  assert!(!kinds.iter().any(|k| matches!(k, TokenKind::JsxOpenTagStart)), "got {:?}", kinds);
}

#[test]
fn lt_then_uppercase_is_jsx() {
  let kinds = lex_kinds("<Button />");
  assert_eq!(kinds.first(), Some(&TokenKind::JsxOpenTagStart));
}

#[test]
fn lt_then_lowercase_is_jsx() {
  let kinds = lex_kinds("<div></div>");
  assert_eq!(kinds.first(), Some(&TokenKind::JsxOpenTagStart));
}

#[test]
fn lt_then_slash_is_close() {
  let kinds = lex_kinds("</Foo>");
  assert_eq!(kinds.first(), Some(&TokenKind::JsxCloseTagStart));
}

#[test]
fn fragment_open_and_close() {
  let kinds = lex_kinds("<></>");
  assert_eq!(kinds.first(), Some(&TokenKind::JsxFragmentOpen));
  assert!(kinds.contains(&TokenKind::JsxFragmentClose), "got {:?}", kinds);
}

#[test]
fn lt_in_paragraph_text_does_not_break() {
  let kinds = lex_kinds("a < b is text\n");
  assert!(!kinds.iter().any(|k| matches!(k, TokenKind::JsxOpenTagStart)), "got {:?}", kinds);
}

#[test]
fn member_expression_tag_name() {
  let kinds = lex_kinds("<Nav.Item />");
  assert!(kinds.contains(&TokenKind::JsxTagName), "got {:?}", kinds);
}

#[test]
fn namespaced_tag_name() {
  let kinds = lex_kinds("<svg:circle />");
  assert!(kinds.contains(&TokenKind::JsxTagName), "got {:?}", kinds);
}
