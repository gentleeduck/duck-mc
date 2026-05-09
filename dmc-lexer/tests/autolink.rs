mod common;
use common::lex_kinds;
use dmc_lexer::token::TokenKind;

#[test]
fn lex_https_autolink() {
  let kinds = lex_kinds("see <https://rust-lang.org> here");
  assert!(kinds.iter().any(|k| matches!(k, TokenKind::Autolink)));
}

#[test]
fn lex_http_autolink() {
  let kinds = lex_kinds("<http://example.com>");
  assert!(kinds.iter().any(|k| matches!(k, TokenKind::Autolink)));
}

#[test]
fn lex_jsx_not_autolink() {
  let kinds = lex_kinds("<Button color=\"red\" />");
  assert!(!kinds.iter().any(|k| matches!(k, TokenKind::Autolink)));
}

#[test]
fn lex_url_with_space_falls_through() {
  let kinds = lex_kinds("<not a url>");
  assert!(!kinds.iter().any(|k| matches!(k, TokenKind::Autolink)));
}

#[test]
fn lex_email_autolink() {
  let kinds = lex_kinds("contact <hi@example.com> for info");
  assert!(kinds.iter().any(|k| matches!(k, TokenKind::Autolink)));
}
