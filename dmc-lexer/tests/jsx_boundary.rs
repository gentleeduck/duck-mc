mod common;
use common::*;
use dmc_lexer::token::TokenKind;
use pretty_assertions::assert_eq;

#[test]
fn lt_then_digit_is_text() {
  let kinds = lex_kinds("5 < 10\n");
  assert!(!kinds.iter().any(|k| matches!(k, TokenKind::JsxOpenTagStart)), "should not start JSX, got {:?}", kinds);
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
fn lt_then_gt_fragment_dispatches_to_jsx() {
  // currently lex_jsx_tag does not yet handle fragments — but the dispatcher
  // should at least route into it (L14 will finish frag support).
  let kinds = lex_kinds("<></>");
  assert!(kinds.first() == Some(&TokenKind::JsxOpenTagStart), "expected JsxOpenTagStart, got {:?}", kinds);
}

#[test]
fn lt_in_paragraph_text_does_not_break() {
  let kinds = lex_kinds("a < b is text\n");
  assert!(!kinds.iter().any(|k| matches!(k, TokenKind::JsxOpenTagStart)), "got {:?}", kinds);
}
