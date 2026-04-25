use crate::parser::Parser;
use duck_md_ast::*;
use duck_md_lexer::token::TokenKind;

/// Accumulate inline nodes until a top-level break (HardBreak, SoftBreak, Eof,
/// a new heading, frontmatter start, or a top-level Import/Export statement).
pub(crate) fn collect_inline_until_break(p: &mut Parser) -> Vec<Node> {
    collect_inline(p, &|kind| {
        matches!(
            kind,
            TokenKind::HardBreak
                | TokenKind::SoftBreak
                | TokenKind::Eof
                | TokenKind::Heading(_)
                | TokenKind::FrontmatterStart
                | TokenKind::Import
                | TokenKind::Export
                | TokenKind::JsxCloseTagStart
        )
    })
}

/// Accumulate inline nodes for the body of a single list item. Terminates on
/// the same conditions as `collect_inline_until_break` — list parsing relies
/// on SoftBreak/HardBreak (already in the break set) to delimit one item per
/// line, and the outer list loop handles the next-marker case naturally.
pub(crate) fn collect_inline_for_list_item(p: &mut Parser) -> Vec<Node> {
    collect_inline_until_break(p)
}

/// Collect inline nodes until `stop(kind)` returns true. The stopping token
/// itself is left on the stream (caller decides whether to consume it).
fn collect_inline(p: &mut Parser, stop: &dyn Fn(&TokenKind) -> bool) -> Vec<Node> {
    let mut out = Vec::new();
    while let Some(t) = p.peek() {
        let kind = t.kind.clone();
        if stop(&kind) {
            break;
        }

        match &kind {
            TokenKind::Text => {
                let raw = t.raw.clone();
                p.advance();
                out.push(Node::Text(Text {
                    value: raw,
                    span: default_span(),
                }));
            }
            TokenKind::Whitespace => {
                let raw = t.raw.clone();
                p.advance();
                out.push(Node::Text(Text {
                    value: raw,
                    span: default_span(),
                }));
            }
            TokenKind::Bold(n) => {
                let open_n = *n;
                p.advance(); // opener
                let inner = collect_inline(p, &|k| {
                    is_top_level_break(k)
                        || matches!(k, TokenKind::Bold(m) if *m == open_n)
                });
                if matches!(p.peek_kind(), Some(TokenKind::Bold(m)) if *m == open_n) {
                    p.advance();
                }
                out.push(Node::Bold(Inline {
                    children: inner,
                    span: default_span(),
                }));
            }
            TokenKind::Italic(n) => {
                let open_n = *n;
                p.advance(); // opener
                let inner = collect_inline(p, &|k| {
                    is_top_level_break(k)
                        || matches!(k, TokenKind::Italic(m) if *m == open_n)
                });
                if matches!(p.peek_kind(), Some(TokenKind::Italic(m)) if *m == open_n) {
                    p.advance();
                }
                out.push(Node::Italic(Inline {
                    children: inner,
                    span: default_span(),
                }));
            }
            TokenKind::Strike(n) => {
                let open_n = *n;
                p.advance(); // opener
                let inner = collect_inline(p, &|k| {
                    is_top_level_break(k)
                        || matches!(k, TokenKind::Strike(m) if *m == open_n)
                });
                if matches!(p.peek_kind(), Some(TokenKind::Strike(m)) if *m == open_n) {
                    p.advance();
                }
                out.push(Node::Strikethrough(Inline {
                    children: inner,
                    span: default_span(),
                }));
            }
            TokenKind::CodeStart(n) => {
                let open_n = *n;
                p.advance(); // opener
                let mut value = String::new();
                while let Some(tok) = p.peek() {
                    match &tok.kind {
                        TokenKind::CodeEnd(m) if *m == open_n => {
                            p.advance();
                            break;
                        }
                        TokenKind::Eof => break,
                        _ => {
                            value.push_str(&tok.raw);
                            p.advance();
                        }
                    }
                }
                out.push(Node::InlineCode(InlineCode {
                    value,
                    span: default_span(),
                }));
            }
            TokenKind::Bracket => {
                let start = p.pos;
                p.advance(); // [
                let inner = collect_inline(p, &|k| {
                    matches!(
                        k,
                        TokenKind::Bracket
                            | TokenKind::HardBreak
                            | TokenKind::SoftBreak
                            | TokenKind::Eof
                    )
                });
                if !matches!(p.peek_kind(), Some(TokenKind::Bracket)) {
                    // not closed → roll back, emit literal `[`
                    p.pos = start;
                    out.push(Node::Text(Text {
                        value: "[".into(),
                        span: default_span(),
                    }));
                    p.advance();
                    continue;
                }
                p.advance(); // ]
                let mut href = String::new();
                if matches!(p.peek_kind(), Some(TokenKind::ParenOpen)) {
                    p.advance();
                    while let Some(tok) = p.peek() {
                        match &tok.kind {
                            TokenKind::ParenClose => {
                                p.advance();
                                break;
                            }
                            TokenKind::Eof => break,
                            _ => {
                                href.push_str(&tok.raw);
                                p.advance();
                            }
                        }
                    }
                }
                out.push(Node::Link(Link {
                    href,
                    title: None,
                    children: inner,
                    span: default_span(),
                }));
            }
            TokenKind::Bang => {
                let start = p.pos;
                p.advance(); // !
                if !matches!(p.peek_kind(), Some(TokenKind::Bracket)) {
                    p.pos = start;
                    out.push(Node::Text(Text {
                        value: "!".into(),
                        span: default_span(),
                    }));
                    p.advance();
                    continue;
                }
                p.advance(); // [
                let mut alt = String::new();
                while let Some(tok) = p.peek() {
                    match &tok.kind {
                        TokenKind::Bracket => {
                            p.advance();
                            break;
                        }
                        TokenKind::Eof | TokenKind::HardBreak | TokenKind::SoftBreak => break,
                        _ => {
                            alt.push_str(&tok.raw);
                            p.advance();
                        }
                    }
                }
                let mut src = String::new();
                if matches!(p.peek_kind(), Some(TokenKind::ParenOpen)) {
                    p.advance();
                    while let Some(tok) = p.peek() {
                        match &tok.kind {
                            TokenKind::ParenClose => {
                                p.advance();
                                break;
                            }
                            TokenKind::Eof => break,
                            _ => {
                                src.push_str(&tok.raw);
                                p.advance();
                            }
                        }
                    }
                }
                out.push(Node::Image(Image {
                    src,
                    alt,
                    title: None,
                    span: default_span(),
                }));
            }
            TokenKind::JsxOpenTagStart => {
                out.push(crate::jsx::parse_jsx(p));
                continue;
            }
            TokenKind::ExpressionStart => {
                out.push(crate::jsx::parse_jsx_expression(p));
                continue;
            }
            TokenKind::MarkdownCommentStart => {
                // skip
                while let Some(t) = p.peek() {
                    match &t.kind {
                        TokenKind::MarkdownCommentEnd => {
                            p.advance();
                            break;
                        }
                        TokenKind::Eof => break,
                        _ => {
                            p.advance();
                        }
                    }
                }
                continue;
            }
            // unknown inline → consume as raw text fallback
            _ => {
                let raw = t.raw.clone();
                p.advance();
                if !raw.is_empty() {
                    out.push(Node::Text(Text {
                        value: raw,
                        span: default_span(),
                    }));
                }
            }
        }
    }
    out
}

fn is_top_level_break(k: &TokenKind) -> bool {
    matches!(
        k,
        TokenKind::HardBreak
            | TokenKind::SoftBreak
            | TokenKind::Eof
            | TokenKind::Heading(_)
            | TokenKind::FrontmatterStart
            | TokenKind::Import
            | TokenKind::Export
            | TokenKind::JsxCloseTagStart
    )
}
