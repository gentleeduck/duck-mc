use duck_md_ast::*;
use duck_md_lexer::token::TokenKind;
use crate::parser::Parser;
use crate::inline::collect_inline_until_break;

pub(crate) fn parse_block(p: &mut Parser) -> Option<Node> {
    match p.peek_kind()? {
        TokenKind::FrontmatterStart => Some(parse_frontmatter(p)),
        TokenKind::Import => Some(consume_import(p)),
        TokenKind::Export => Some(consume_export(p)),
        TokenKind::Heading(_) => Some(parse_heading(p)),
        TokenKind::HardBreak | TokenKind::SoftBreak => {
            p.advance();
            None
        }
        _ => Some(parse_paragraph(p)),
    }
}

fn parse_frontmatter(p: &mut Parser) -> Node {
    p.advance(); // start
    let raw = match p.peek() {
        Some(t) if matches!(t.kind, TokenKind::FrontmatterContent) => {
            let raw = t.raw.clone();
            p.advance();
            raw
        }
        _ => String::new(),
    };
    if matches!(p.peek_kind(), Some(TokenKind::FrontmatterEnd)) {
        p.advance();
    }
    let data = serde_yaml::from_str::<serde_json::Value>(&raw)
        .unwrap_or(serde_json::Value::Null);
    Node::Frontmatter(Frontmatter { raw, data, span: default_span() })
}

fn consume_import(p: &mut Parser) -> Node {
    let raw = p.peek().map(|t| t.raw.clone()).unwrap_or_default();
    p.advance();
    Node::Import(Import { raw, span: default_span() })
}

fn consume_export(p: &mut Parser) -> Node {
    let raw = p.peek().map(|t| t.raw.clone()).unwrap_or_default();
    p.advance();
    Node::Export(Export { raw, span: default_span() })
}

fn parse_heading(p: &mut Parser) -> Node {
    let level = match p.peek_kind() {
        Some(TokenKind::Heading(n)) => *n,
        _ => 1,
    };
    p.advance();
    let children = collect_inline_until_break(p);
    let plain = plain_text(&children);
    let id = slug::slugify(&plain);
    Node::Heading(Heading {
        level,
        id,
        children,
        span: default_span(),
    })
}

fn parse_paragraph(p: &mut Parser) -> Node {
    let children = collect_inline_until_break(p);
    Node::Paragraph(Paragraph {
        children,
        span: default_span(),
    })
}

fn plain_text(nodes: &[Node]) -> String {
    let mut s = String::new();
    for n in nodes {
        match n {
            Node::Text(t) => s.push_str(&t.value),
            Node::Bold(i) | Node::Italic(i) | Node::Strikethrough(i) => s.push_str(&plain_text(&i.children)),
            Node::InlineCode(c) => s.push_str(&c.value),
            _ => {}
        }
    }
    s.trim().to_string()
}
