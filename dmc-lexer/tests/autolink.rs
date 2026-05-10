mod common;
use common::lex_kinds;
use dmc_lexer::token::{AutolinkKind, TokenKind};

#[test]
fn https_angle_autolink() {
  let kinds = lex_kinds("see <https://rust-lang.org> here");
  assert!(kinds.iter().any(|k| matches!(k, TokenKind::Autolink(AutolinkKind::AngleUrl))), "got {:?}", kinds);
}

#[test]
fn http_angle_autolink() {
  let kinds = lex_kinds("<http://example.com>");
  assert!(kinds.iter().any(|k| matches!(k, TokenKind::Autolink(AutolinkKind::AngleUrl))), "got {:?}", kinds);
}

#[test]
fn jsx_self_closing_is_not_autolink() {
  let kinds = lex_kinds("<Button color=\"red\" />");
  assert!(!kinds.iter().any(|k| matches!(k, TokenKind::Autolink(_))), "got {:?}", kinds);
}

#[test]
fn angle_with_space_falls_through() {
  let kinds = lex_kinds("<not a url>");
  assert!(!kinds.iter().any(|k| matches!(k, TokenKind::Autolink(_))), "got {:?}", kinds);
}

#[test]
fn email_angle_autolink() {
  let kinds = lex_kinds("contact <hi@example.com> for info");
  assert!(kinds.iter().any(|k| matches!(k, TokenKind::Autolink(AutolinkKind::AngleEmail))), "got {:?}", kinds);
}

#[test]
fn bare_https_url() {
  let kinds = lex_kinds("see https://example.com here");
  assert!(kinds.iter().any(|k| matches!(k, TokenKind::Autolink(AutolinkKind::BareUrl))), "got {:?}", kinds);
}

#[test]
fn bare_www_url() {
  let kinds = lex_kinds("try www.example.com today");
  assert!(kinds.iter().any(|k| matches!(k, TokenKind::Autolink(AutolinkKind::BareWww))), "got {:?}", kinds);
}

#[test]
fn bare_url_strips_trailing_punct() {
  // The URL token shouldn't include the trailing period.
  let pairs: Vec<_> =
    lex_kinds("see https://example.com.").into_iter().filter(|k| matches!(k, TokenKind::Autolink(_))).collect();
  assert_eq!(pairs.len(), 1);
}

#[test]
fn httpsa_is_not_autolink() {
  let kinds = lex_kinds("httpsa://nope.com");
  assert!(!kinds.iter().any(|k| matches!(k, TokenKind::Autolink(_))), "got {:?}", kinds);
}

#[test]
fn unknown_scheme_still_autolink() {
  // CM 6.5 allows any valid scheme.
  let kinds = lex_kinds("<irc://chat.example.com>");
  assert!(kinds.iter().any(|k| matches!(k, TokenKind::Autolink(AutolinkKind::AngleUrl))), "got {:?}", kinds);
}

#[test]
fn scheme_with_plus_dot_dash() {
  let kinds = lex_kinds("<x-custom.scheme+1://body>");
  assert!(kinds.iter().any(|k| matches!(k, TokenKind::Autolink(AutolinkKind::AngleUrl))), "got {:?}", kinds);
}

#[test]
fn scheme_too_short_is_not_autolink() {
  // Single-letter scheme rejected (CM requires 2-32 chars).
  let kinds = lex_kinds("<a:body>");
  assert!(!kinds.iter().any(|k| matches!(k, TokenKind::Autolink(_))), "got {:?}", kinds);
}

#[test]
fn scheme_starting_with_digit_rejected() {
  let kinds = lex_kinds("<1http://example.com>");
  assert!(!kinds.iter().any(|k| matches!(k, TokenKind::Autolink(_))), "got {:?}", kinds);
}

#[test]
fn empty_body_after_scheme_rejected() {
  let kinds = lex_kinds("<http:>");
  assert!(!kinds.iter().any(|k| matches!(k, TokenKind::Autolink(_))), "got {:?}", kinds);
}
