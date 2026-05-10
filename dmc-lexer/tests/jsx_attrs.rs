mod common;
use common::*;
use dmc_lexer::token::{QuoteKind, TokenKind};
use pretty_assertions::assert_eq;

#[test]
fn string_attr_double() {
  let kinds = lex_kinds("<Btn color=\"red\" />");
  assert!(kinds.contains(&TokenKind::JsxAttributeName));
  assert!(kinds.contains(&TokenKind::JsxAttrEq));
  assert!(kinds.contains(&TokenKind::JsxAttrString));
  assert!(kinds.contains(&TokenKind::JsxAttrStringOpen(QuoteKind::Double)));
  assert!(kinds.contains(&TokenKind::JsxAttrStringClose(QuoteKind::Double)));
}

#[test]
fn string_attr_single() {
  let kinds = lex_kinds("<Btn color='red' />");
  assert!(kinds.contains(&TokenKind::JsxAttrStringOpen(QuoteKind::Single)), "got {:?}", kinds);
  assert!(kinds.contains(&TokenKind::JsxAttrStringClose(QuoteKind::Single)), "got {:?}", kinds);
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
  assert!(!kinds.contains(&TokenKind::JsxAttrEq), "no Eq for boolean: {:?}", kinds);
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
  assert!(kinds.contains(&TokenKind::JsxSelfClosingEnd), "expected JsxSelfClosingEnd; got {:?}", kinds);
  // Make sure `/` and `>` weren't separately tokenized as something stray.
  assert!(
    !kinds.iter().any(|k| matches!(k, TokenKind::BlockQuoteMarker)),
    "stray BlockQuoteMarker means / > were tokenized separately; got {:?}",
    kinds
  );
}

#[test]
fn self_closing_no_attrs_still_works() {
  let kinds = lex_kinds("<Foo />");
  assert!(kinds.contains(&TokenKind::JsxSelfClosingEnd), "got {:?}", kinds);
}

#[test]
fn spread_attribute() {
  let kinds = lex_kinds("<Btn {...rest} />");
  assert!(kinds.contains(&TokenKind::JsxAttributeSpread), "got {:?}", kinds);
}

#[test]
fn escaped_quote_in_string() {
  // `\"` inside the value must not terminate the string early.
  let kinds = lex_kinds(r#"<Btn x="say \"hi\"" />"#);
  let opens = kinds.iter().filter(|k| matches!(k, TokenKind::JsxAttrStringOpen(_))).count();
  let closes = kinds.iter().filter(|k| matches!(k, TokenKind::JsxAttrStringClose(_))).count();
  assert_eq!(opens, 1);
  assert_eq!(closes, 1);
}
