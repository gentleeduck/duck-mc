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
fn import_body_stays_verbatim_across_lines_and_strings() {
  let src = "import {\n  A,\n  B,\n} from \"./{brace}.js\"\n# H\n";
  let pairs = lex_pairs(src);
  assert_eq!(pairs[0], (TokenKind::Import, "import {\n  A,\n  B,\n} from \"./{brace}.js\"".to_string()));
  assert_eq!(pairs[2], (TokenKind::Heading(1), "#".to_string()));
  assert!(
    !pairs[..2].iter().any(|(kind, _)| {
      matches!(
        kind,
        TokenKind::Heading(_)
          | TokenKind::LinkOpen
          | TokenKind::LinkClose
          | TokenKind::ExpressionStart
          | TokenKind::ExpressionEnd
          | TokenKind::UnorderedListMarker
          | TokenKind::OrderedListMarker(_)
      )
    }),
    "got {:?}",
    pairs
  );
}

#[test]
fn export_body_ignores_markdownish_strings() {
  let src = "export const z = { text: \"# not heading\", link: \"[x](y)\", braces: \"{}\" }\n* list\n";
  let pairs = lex_pairs(src);
  assert_eq!(
    pairs[0],
    (
      TokenKind::Export,
      "export const z = { text: \"# not heading\", link: \"[x](y)\", braces: \"{}\" }".to_string()
    )
  );
  assert_eq!(pairs[2], (TokenKind::UnorderedListMarker, "* ".to_string()));
  assert!(
    !pairs[..2].iter().any(|(kind, _)| {
      matches!(
        kind,
        TokenKind::Heading(_)
          | TokenKind::LinkOpen
          | TokenKind::LinkClose
          | TokenKind::ExpressionStart
          | TokenKind::ExpressionEnd
      )
    }),
    "got {:?}",
    pairs
  );
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

#[test]
fn import_keyword_inside_quoted_prose_is_not_esm() {
  let kinds = lex_kinds("\"import { x } from 'y'\"\n");
  assert!(!kinds.contains(&TokenKind::Import), "got {:?}", kinds);
}
