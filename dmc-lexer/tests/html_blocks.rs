mod common;
use common::*;
use dmc_lexer::token::{HtmlBlockKind, TokenKind};

#[test]
fn processing_instruction() {
  let kinds = lex_kinds("<?xml version=\"1.0\"?>\n");
  assert!(kinds.contains(&TokenKind::HtmlBlockOpen(HtmlBlockKind::Type3)), "got {:?}", kinds);
  assert!(kinds.contains(&TokenKind::HtmlBlockClose));
}

#[test]
fn doctype_declaration() {
  let kinds = lex_kinds("<!DOCTYPE html>\n");
  assert!(kinds.contains(&TokenKind::HtmlBlockOpen(HtmlBlockKind::Type4)), "got {:?}", kinds);
  assert!(kinds.contains(&TokenKind::HtmlBlockClose));
}

#[test]
fn cdata_section() {
  let kinds = lex_kinds("<![CDATA[ raw <stuff> ]]>\n");
  assert!(kinds.contains(&TokenKind::HtmlBlockOpen(HtmlBlockKind::Type5)), "got {:?}", kinds);
  assert!(kinds.contains(&TokenKind::HtmlBlockClose));
}

#[test]
fn declaration_lowercase_is_not_type4() {
  // `<!doctype>` lowercase doesn't satisfy CM type 4 (needs uppercase).
  let kinds = lex_kinds("<!doctype html>\n");
  assert!(!kinds.contains(&TokenKind::HtmlBlockOpen(HtmlBlockKind::Type4)), "got {:?}", kinds);
}

#[test]
fn pi_unterminated_does_not_panic() {
  let _ = lex_kinds("<?never closes\n");
}

#[test]
fn cdata_unterminated_does_not_panic() {
  let _ = lex_kinds("<![CDATA[never closes\n");
}
