use duck_md_ast::*;
use duck_md_lexer::token::{Token, TokenKind};
use crate::parser::Parser;

pub(crate) fn collect_inline_until_break(p: &mut Parser) -> Vec<Node> {
    let mut out = Vec::new();
    while let Some(t) = p.peek() {
        match &t.kind {
            TokenKind::HardBreak | TokenKind::SoftBreak | TokenKind::Eof | TokenKind::Heading(_) => break,
            TokenKind::Text => {
                let raw = t.raw.clone();
                p.advance();
                out.push(Node::Text(Text { value: raw, span: default_span() }));
            }
            TokenKind::Whitespace => {
                let raw = t.raw.clone();
                p.advance();
                out.push(Node::Text(Text { value: raw, span: default_span() }));
            }
            TokenKind::CodeStart(_) => {
                // skip the start; consume inner Text then end (best-effort)
                p.advance();
                if let Some(Token { kind: TokenKind::Text, raw, .. }) = p.peek().cloned() {
                    p.advance();
                    out.push(Node::InlineCode(InlineCode { value: raw, span: default_span() }));
                }
                if matches!(p.peek_kind(), Some(TokenKind::CodeEnd(_))) {
                    p.advance();
                }
            }
            // unknown inline → consume as raw text fallback
            _ => {
                let raw = t.raw.clone();
                p.advance();
                if !raw.is_empty() {
                    out.push(Node::Text(Text { value: raw, span: default_span() }));
                }
            }
        }
    }
    out
}
