mod common;
use common::*;
use duck_md_lexer::token::TokenKind;
use pretty_assertions::assert_eq;

#[test]
fn string_attr_double() {
  let kinds = lex_kinds("<Btn color=\"red\" />");
  assert!(kinds.contains(&TokenKind::JsxAttributeName));
  assert!(kinds.contains(&TokenKind::Eq));
  assert!(kinds.contains(&TokenKind::String));
}

#[test]
fn string_attr_single() {
  let kinds = lex_kinds("<Btn color='red' />");
  assert!(kinds.contains(&TokenKind::String), "got {:?}", kinds);
}

#[test]
fn expression_attr() {
  let kinds = lex_kinds("<Btn onClick={handle} />");
  assert!(kinds.contains(&TokenKind::ExpressionStart), "got {:?}", kinds);
  assert!(kinds.contains(&TokenKind::ExpressionEnd), "got {:?}", kinds);
}

#[test]
fn expression_attr_nested_braces() {
  let kinds = lex_kinds("<Btn data={{a:1}} />");
  let starts = kinds.iter().filter(|k| matches!(k, TokenKind::ExpressionStart)).count();
  let ends = kinds.iter().filter(|k| matches!(k, TokenKind::ExpressionEnd)).count();
  assert_eq!(starts, 1, "balanced: {:?}", kinds);
  assert_eq!(ends, 1, "balanced: {:?}", kinds);
}

#[test]
fn boolean_attr() {
  let kinds = lex_kinds("<Btn disabled />");
  assert!(kinds.contains(&TokenKind::JsxAttributeName));
  assert!(!kinds.contains(&TokenKind::Eq), "no Eq for boolean: {:?}", kinds);
}

#[test]
fn data_attr_with_dash() {
  let kinds = lex_kinds("<div data-slot=\"trigger\" />");
  assert!(kinds.contains(&TokenKind::JsxAttributeName), "got {:?}", kinds);
}

#[test]
fn aria_attr_with_dash() {
  let kinds = lex_kinds("<div aria-label=\"x\" />");
  assert!(kinds.contains(&TokenKind::JsxAttributeName), "got {:?}", kinds);
}

#[test]
fn self_closing_after_attrs_emits_self_closing_end() {
  let kinds = lex_kinds("<Btn color=\"red\" />");
  assert!(
    kinds.contains(&TokenKind::JsxSelfClosingEnd),
    "expected JsxSelfClosingEnd; got {:?}",
    kinds
  );
  assert!(
    !kinds.iter().any(|k| matches!(k, TokenKind::BlockQuote)),
    "stray BlockQuote means / > were tokenized separately; got {:?}",
    kinds
  );
}

#[test]
fn self_closing_no_attrs_still_works() {
  let kinds = lex_kinds("<Foo />");
  assert!(kinds.contains(&TokenKind::JsxSelfClosingEnd), "got {:?}", kinds);
}
