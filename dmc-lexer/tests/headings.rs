mod common;
use common::*;
use dmc_lexer::token::TokenKind;

#[test]
fn h1_through_h6() {
  for level in 1u8..=6 {
    let src = format!("{} hi\n", "#".repeat(level as usize));
    let kinds = lex_kinds(&src);
    assert!(kinds.contains(&TokenKind::Heading(level)), "level {}: {:?}", level, kinds);
  }
}

#[test]
fn seven_hashes_is_text() {
  let kinds = lex_kinds("####### nope\n");
  assert!(!kinds.iter().any(|k| matches!(k, TokenKind::Heading(_))), "got {:?}", kinds);
}

#[test]
fn hashtag_without_space_is_text() {
  let kinds = lex_kinds("#hashtag\n");
  assert!(!kinds.iter().any(|k| matches!(k, TokenKind::Heading(_))), "got {:?}", kinds);
}

#[test]
fn trailing_hashes_emitted() {
  let kinds = lex_kinds("# title #\n");
  assert!(kinds.contains(&TokenKind::Heading(1)));
  assert!(kinds.contains(&TokenKind::HeadingTrailingHashes), "got {:?}", kinds);
}

#[test]
fn trailing_hashes_count_can_differ() {
  let kinds = lex_kinds("### title ####\n");
  assert!(kinds.contains(&TokenKind::Heading(3)));
  assert!(kinds.contains(&TokenKind::HeadingTrailingHashes));
}

#[test]
fn no_space_before_trailing_hash_is_text() {
  let kinds = lex_kinds("# title#\n");
  assert!(!kinds.contains(&TokenKind::HeadingTrailingHashes), "got {:?}", kinds);
}

#[test]
fn setext_h1_underline() {
  use dmc_lexer::token::SetextLevel;
  let kinds = lex_kinds("Title\n=====\n");
  assert!(kinds.contains(&TokenKind::SetextUnderline(SetextLevel::H1)), "got {:?}", kinds);
}
