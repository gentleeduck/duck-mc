mod common;
use common::*;
use dmc_lexer::token::TokenKind;
use pretty_assertions::assert_eq;

#[test]
fn simple_expression() {
  let kinds = lex_kinds("Hello {name}\n");
  assert!(kinds.contains(&TokenKind::ExpressionStart), "got {:?}", kinds);
  assert!(kinds.contains(&TokenKind::ExpressionEnd), "got {:?}", kinds);
}

#[test]
fn expression_with_arithmetic() {
  let kinds = lex_kinds("{2 + 2}");
  assert!(kinds.contains(&TokenKind::ExpressionStart));
  assert!(kinds.contains(&TokenKind::ExpressionEnd));
}

#[test]
fn nested_expression_balances_braces() {
  let kinds = lex_kinds("{a ? {b: 1} : null}");
  let starts = kinds.iter().filter(|k| matches!(k, TokenKind::ExpressionStart)).count();
  let ends = kinds.iter().filter(|k| matches!(k, TokenKind::ExpressionEnd)).count();
  assert_eq!(starts, 1);
  assert_eq!(ends, 1);
}

#[test]
fn string_with_brace_does_not_terminate_early() {
  // The `}` inside the string should NOT close the expression.
  let kinds = lex_kinds(r#"{x = "}"}"#);
  let starts = kinds.iter().filter(|k| matches!(k, TokenKind::ExpressionStart)).count();
  let ends = kinds.iter().filter(|k| matches!(k, TokenKind::ExpressionEnd)).count();
  assert_eq!(starts, 1);
  assert_eq!(ends, 1);
}

#[test]
fn template_literal_inside_expression() {
  let kinds = lex_kinds("{`hello ${name}`}");
  assert!(kinds.contains(&TokenKind::ExpressionStart));
  assert!(kinds.contains(&TokenKind::ExpressionEnd));
}

#[test]
fn unterminated_expression_does_not_panic() {
  // Body is emitted as text; no closing token. Must not panic or loop.
  let _ = lex_kinds("{oops\n");
}
