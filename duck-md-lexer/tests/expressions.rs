mod common;
use common::*;
use duck_md_lexer::token::TokenKind;
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
fn nested_expression() {
    let kinds = lex_kinds("{a ? {b: 1} : null}");
    let starts = kinds.iter().filter(|k| matches!(k, TokenKind::ExpressionStart)).count();
    let ends = kinds.iter().filter(|k| matches!(k, TokenKind::ExpressionEnd)).count();
    assert_eq!(starts, 1);
    assert_eq!(ends, 1);
}

#[test]
fn unterminated_expression_does_not_panic() {
    // do not assert; just ensure no panic / infinite loop
    let _ = lex_kinds("{oops\n");
}
