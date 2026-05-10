mod common;
use common::*;
use dmc_lexer::token::TokenKind;
use pretty_assertions::assert_eq;

#[test]
fn crlf_treated_as_single_newline() {
  let lf = lex_kinds("a\nb\n");
  let crlf = lex_kinds("a\r\nb\r\n");
  assert_eq!(lf, crlf);
}

#[test]
fn lone_cr_treated_as_newline() {
  let lf = lex_kinds("a\nb\n");
  let cr = lex_kinds("a\rb\r");
  assert_eq!(lf, cr);
}

#[test]
fn crlf_blank_line() {
  let kinds = lex_kinds("a\r\n\r\nb\r\n");
  assert!(kinds.contains(&TokenKind::BlankLine), "got {:?}", kinds);
}

#[test]
fn tab_advances_to_next_stop() {
  // `\t` at column 0 -> column 4. Followed by content, this is an
  // indented code block.
  let kinds = lex_kinds("\n\n\tcode\n");
  assert!(kinds.contains(&TokenKind::IndentedCodeLine), "got {:?}", kinds);
}

#[test]
fn tab_after_chars_jumps_to_multiple_of_four() {
  // `ab\tc` -- after `ab` column is 2, tab jumps to 4.
  let kinds = lex_kinds("ab\tc\n");
  // No structural assertion; just make sure it doesn't panic and ends
  // with EOF.
  assert!(matches!(kinds.last(), Some(TokenKind::Eof)));
}
