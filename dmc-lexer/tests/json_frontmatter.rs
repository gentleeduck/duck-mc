mod common;
use common::*;
use dmc_lexer::token::{FrontmatterKind, TokenKind};

#[test]
fn json_frontmatter_minimal() {
  let src = "{\"title\": \"hi\"}\n# Heading\n";
  let kinds = lex_kinds(src);
  assert!(kinds.contains(&TokenKind::FrontmatterStart(FrontmatterKind::Json)), "got {:?}", kinds);
  assert!(kinds.contains(&TokenKind::FrontmatterEnd(FrontmatterKind::Json)));
  assert!(kinds.contains(&TokenKind::Heading(1)));
}

#[test]
fn json_frontmatter_multiline() {
  let src = "{\n  \"title\": \"hi\",\n  \"tags\": [\"a\", \"b\"]\n}\nbody\n";
  let kinds = lex_kinds(src);
  assert!(kinds.contains(&TokenKind::FrontmatterStart(FrontmatterKind::Json)));
  assert!(kinds.contains(&TokenKind::FrontmatterEnd(FrontmatterKind::Json)));
}

#[test]
fn json_frontmatter_with_nested_object() {
  let src = "{\"a\": {\"b\": 1}}\n";
  let kinds = lex_kinds(src);
  assert!(kinds.contains(&TokenKind::FrontmatterStart(FrontmatterKind::Json)), "got {:?}", kinds);
  assert!(kinds.contains(&TokenKind::FrontmatterEnd(FrontmatterKind::Json)));
}

#[test]
fn json_frontmatter_with_brace_in_string() {
  // `}` inside a string should not close early.
  let src = "{\"x\": \"a}b\"}\n";
  let kinds = lex_kinds(src);
  let starts = kinds.iter().filter(|k| matches!(k, TokenKind::FrontmatterStart(_))).count();
  let ends = kinds.iter().filter(|k| matches!(k, TokenKind::FrontmatterEnd(_))).count();
  assert_eq!(starts, 1);
  assert_eq!(ends, 1);
}

#[test]
fn brace_not_at_file_start_is_expression() {
  // Leading text -> the `{` becomes an MDX expression, not frontmatter.
  let src = "hi\n{name}\n";
  let kinds = lex_kinds(src);
  assert!(!kinds.iter().any(|k| matches!(k, TokenKind::FrontmatterStart(_))), "got {:?}", kinds);
  assert!(kinds.contains(&TokenKind::ExpressionStart));
}

#[test]
fn unbalanced_json_falls_through() {
  // Missing close brace: parser should see no frontmatter, just an
  // unterminated MDX expression.
  let src = "{\"unclosed\": true\n";
  let kinds = lex_kinds(src);
  assert!(!kinds.iter().any(|k| matches!(k, TokenKind::FrontmatterStart(_))), "got {:?}", kinds);
}
