use crate::ast::*;
use duck_md_lexer::token::TokenKind;
use crate::parser::Parser;

/// Caller is positioned at `JsxOpenTagStart`. Consumes through the matching
/// close (or self-close), returns either a JsxElement, JsxSelfClosing, or
/// JsxFragment node.
pub(crate) fn parse_jsx(p: &mut Parser) -> Node {
    p.advance(); // JsxOpenTagStart
    let name = if let Some(t) = p.peek() {
        if matches!(t.kind, TokenKind::JsxTagName) {
            let n = t.raw.clone();
            p.advance();
            n
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    let attrs = parse_attrs(p);

    // Determine end of opening tag
    match p.peek_kind() {
        Some(TokenKind::JsxSelfClosingEnd) => {
            p.advance();
            return Node::JsxSelfClosing(JsxSelfClosing {
                name,
                attrs,
                span: default_span(),
            });
        }
        Some(TokenKind::JsxOpenTagEnd) => {
            p.advance();
        }
        _ => {
            p.warn(format!("unterminated JSX open tag <{name}> — synthesizing self-close"));
            return Node::JsxSelfClosing(JsxSelfClosing {
                name,
                attrs,
                span: default_span(),
            });
        }
    }

    // Children: re-enter block parsing until matching close tag
    let mut children = Vec::new();
    loop {
        match p.peek_kind() {
            Some(TokenKind::JsxCloseTagStart) => {
                p.advance();
                // optional matching name
                if matches!(p.peek_kind(), Some(TokenKind::JsxTagName)) {
                    p.advance();
                }
                if matches!(p.peek_kind(), Some(TokenKind::JsxCloseTagEnd)) {
                    p.advance();
                }
                break;
            }
            Some(TokenKind::Eof) | None => break,
            _ => {
                let before = p.pos;
                if let Some(node) = crate::block::parse_block(p) {
                    children.push(node);
                }
                if p.pos == before {
                    p.advance();
                }
            }
        }
    }

    if name.is_empty() {
        Node::JsxFragment(JsxFragment {
            children,
            span: default_span(),
        })
    } else {
        Node::JsxElement(JsxElement {
            name,
            attrs,
            children,
            span: default_span(),
        })
    }
}

fn parse_attrs(p: &mut Parser) -> Vec<JsxAttr> {
    let mut out = Vec::new();
    while let Some(TokenKind::JsxAttributeName) = p.peek_kind() {
        let name = p.peek().unwrap().raw.clone();
        p.advance();
        let value = if matches!(p.peek_kind(), Some(TokenKind::Eq)) {
            p.advance();
            match p.peek_kind() {
                Some(TokenKind::String) => {
                    let s = p.peek().unwrap().raw.clone();
                    p.advance();
                    JsxAttrValue::String(s)
                }
                Some(TokenKind::ExpressionStart) => {
                    p.advance();
                    let mut s = String::new();
                    while let Some(t) = p.peek() {
                        match &t.kind {
                            TokenKind::ExpressionEnd | TokenKind::Eof => break,
                            _ => {
                                s.push_str(&t.raw);
                                p.advance();
                            }
                        }
                    }
                    if matches!(p.peek_kind(), Some(TokenKind::ExpressionEnd)) {
                        p.advance();
                    }
                    JsxAttrValue::Expression(s)
                }
                _ => JsxAttrValue::Boolean,
            }
        } else {
            JsxAttrValue::Boolean
        };
        out.push(JsxAttr {
            name,
            value,
            span: default_span(),
        });
    }
    out
}

/// Standalone `{expr}` expression. Caller positioned at ExpressionStart.
pub(crate) fn parse_jsx_expression(p: &mut Parser) -> Node {
    p.advance(); // ExpressionStart
    let mut s = String::new();
    while let Some(t) = p.peek() {
        match &t.kind {
            TokenKind::ExpressionEnd | TokenKind::Eof => break,
            _ => {
                s.push_str(&t.raw);
                p.advance();
            }
        }
    }
    if matches!(p.peek_kind(), Some(TokenKind::ExpressionEnd)) {
        p.advance();
    }
    Node::JsxExpression(JsxExpression {
        value: s,
        span: default_span(),
    })
}

/// Skip a markdown comment `{/* ... */}`. Caller is positioned at MarkdownCommentStart.
pub(crate) fn skip_md_comment(p: &mut Parser) {
    p.advance(); // MarkdownCommentStart
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
}
