mod common;
use common::*;
use duck_md_lexer::token::TokenKind;
use pretty_assertions::assert_eq;

#[test]
fn markdown_comment_basic() {
    let kinds = lex_kinds("{/* hidden */}");
    assert!(kinds.contains(&TokenKind::MarkdownCommentStart), "got {:?}", kinds);
    assert!(kinds.contains(&TokenKind::MarkdownCommentEnd), "got {:?}", kinds);
}

#[test]
fn markdown_comment_with_text_around() {
    let kinds = lex_kinds("hi {/* x */} bye");
    assert!(kinds.contains(&TokenKind::MarkdownCommentStart));
    assert!(kinds.contains(&TokenKind::MarkdownCommentEnd));
}

#[test]
fn markdown_comment_multiline() {
    let src = "{/* line1\nline2 */}";
    let kinds = lex_kinds(src);
    assert!(kinds.contains(&TokenKind::MarkdownCommentStart));
    assert!(kinds.contains(&TokenKind::MarkdownCommentEnd));
}
