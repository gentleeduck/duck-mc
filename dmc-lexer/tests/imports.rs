mod common;
use common::*;
use dmc_lexer::token::TokenKind;
use pretty_assertions::assert_eq;

#[test]
fn single_line_import() {
  let kinds = lex_kinds("import { Button } from './x'\n");
  assert_eq!(kinds.first(), Some(&TokenKind::Import));
}

#[test]
fn export_const() {
  let kinds = lex_kinds("export const x = 1\n");
  assert_eq!(kinds.first(), Some(&TokenKind::Export));
}

#[test]
fn multi_line_import_with_braces() {
  let src = "import {\n  A,\n  B,\n} from './x'\n";
  let kinds = lex_kinds(src);
  assert_eq!(kinds.first(), Some(&TokenKind::Import));
}

#[test]
fn export_function_with_template_literal() {
  let src = "export function greet(name) {\n  return `hello ${name}`\n}\n";
  let kinds = lex_kinds(src);
  assert_eq!(kinds.first(), Some(&TokenKind::Export));
}

#[test]
fn import_inside_paragraph_is_text() {
  let kinds = lex_kinds("hello import x\n");
  assert!(!kinds.contains(&TokenKind::Import), "got {:?}", kinds);
}

#[test]
fn important_word_not_import() {
  let kinds = lex_kinds("important: not an import\n");
  assert!(!kinds.contains(&TokenKind::Import), "got {:?}", kinds);
}

#[test]
fn indented_import_not_esm() {
  let kinds = lex_kinds("  import x\n");
  assert!(!kinds.contains(&TokenKind::Import), "got {:?}", kinds);
}
