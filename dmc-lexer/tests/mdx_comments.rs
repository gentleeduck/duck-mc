mod common;
use common::*;
use dmc_lexer::token::TokenKind;

#[test]
fn mdx_comment_basic() {
  let kinds = lex_kinds("{/* hidden */}");
  assert!(kinds.contains(&TokenKind::MdxCommentOpen), "got {:?}", kinds);
  assert!(kinds.contains(&TokenKind::MdxCommentClose), "got {:?}", kinds);
}

#[test]
fn mdx_comment_with_text_around() {
  let kinds = lex_kinds("hi {/* x */} bye");
  assert!(kinds.contains(&TokenKind::MdxCommentOpen));
  assert!(kinds.contains(&TokenKind::MdxCommentClose));
}

#[test]
fn mdx_comment_multiline() {
  let src = "{/* line1\nline2 */}";
  let kinds = lex_kinds(src);
  assert!(kinds.contains(&TokenKind::MdxCommentOpen));
  assert!(kinds.contains(&TokenKind::MdxCommentClose));
}

#[test]
fn mdx_comment_with_stars_inside() {
  let kinds = lex_kinds("{/* a * b ** c */}");
  assert!(kinds.contains(&TokenKind::MdxCommentOpen));
  assert!(kinds.contains(&TokenKind::MdxCommentClose));
}

#[test]
fn unterminated_mdx_comment_does_not_panic() {
  let _ = lex_kinds("{/* never closes\n");
}

#[test]
fn html_comment_basic() {
  let kinds = lex_kinds("Inline <!-- hello --> world");
  assert!(kinds.contains(&TokenKind::HtmlCommentOpen), "got {:?}", kinds);
  assert!(kinds.contains(&TokenKind::HtmlCommentClose), "got {:?}", kinds);
}

#[test]
fn html_comment_multiline() {
  let kinds = lex_kinds("<!-- a\nb\nc -->");
  assert!(kinds.contains(&TokenKind::HtmlCommentOpen));
  assert!(kinds.contains(&TokenKind::HtmlCommentClose));
}
