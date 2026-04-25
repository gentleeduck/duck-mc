use crate::ast::*;
use duck_md_lexer::token::TokenKind;
use crate::parser::Parser;
use crate::inline::collect_inline_until_break;

pub(crate) fn parse_block(p: &mut Parser) -> Option<Node> {
    match p.peek_kind()? {
        TokenKind::FrontmatterStart => Some(parse_frontmatter(p)),
        TokenKind::Import => Some(consume_import(p)),
        TokenKind::Export => Some(consume_export(p)),
        TokenKind::Heading(_) => Some(parse_heading(p)),
        TokenKind::CodeStart(n) if *n >= 3 => Some(parse_code_block(p)),
        TokenKind::JsxOpenTagStart => Some(crate::jsx::parse_jsx(p)),
        TokenKind::ExpressionStart => Some(crate::jsx::parse_jsx_expression(p)),
        TokenKind::MarkdownCommentStart => {
            crate::jsx::skip_md_comment(p);
            None
        }
        TokenKind::UnorderedListItem => Some(parse_list(p, false)),
        TokenKind::OrderedListItem => Some(parse_list(p, true)),
        TokenKind::BlockQuote => Some(parse_blockquote(p)),
        TokenKind::ThematicBreak => {
            p.advance();
            Some(Node::HorizontalRule(HorizontalRule { span: default_span() }))
        }
        TokenKind::HardBreak | TokenKind::SoftBreak => {
            p.advance();
            None
        }
        _ => {
            if let Some(n) = crate::table::try_parse_table(p) {
                return Some(n);
            }
            Some(parse_paragraph(p))
        }
    }
}

fn parse_list(p: &mut Parser, ordered: bool) -> Node {
    let mut items: Vec<Node> = Vec::new();
    let start: Option<u32> = if ordered {
        p.peek()
            .and_then(|t| t.raw.trim_end_matches('.').parse::<u32>().ok())
    } else {
        None
    };

    while let Some(kind) = p.peek_kind() {
        let want_marker = if ordered {
            matches!(kind, TokenKind::OrderedListItem)
        } else {
            matches!(kind, TokenKind::UnorderedListItem)
        };
        if !want_marker {
            break;
        }
        p.advance(); // consume marker

        // For ordered list items the lexer leaves the trailing `.` (and any
        // following space) inside the next Text token (e.g. ". three"). Trim
        // a single leading `.` and any leading ASCII whitespace from that
        // first Text token so the inline content starts with the actual body.
        if ordered
            && let Some(t) = p.peek()
            && matches!(t.kind, TokenKind::Text)
        {
            let raw = t.raw.clone();
            let trimmed = raw.strip_prefix('.').unwrap_or(&raw);
            let trimmed = trimmed.trim_start_matches([' ', '\t']);
            if trimmed.is_empty() {
                // skip the `.` text token entirely
                p.advance();
            } else if trimmed.len() != raw.len() {
                // rewrite the token's raw payload in place
                let pos = p.pos;
                p.tokens[pos].raw = trimmed.to_string();
            }
        }

        items.push(parse_one_list_item(p, ordered));

        // Consume one separator break between items so the next iteration sees
        // the next list marker (or some other block) at the start of the line.
        if matches!(
            p.peek_kind(),
            Some(TokenKind::SoftBreak) | Some(TokenKind::HardBreak)
        ) {
            p.advance();
        }
    }

    Node::List(List {
        ordered,
        start,
        children: items,
        span: default_span(),
    })
}

fn parse_one_list_item(p: &mut Parser, ordered: bool) -> Node {
    // The marker has already been consumed by the caller. Detect a GFM
    // task-list prefix `[ ]` / `[x]` / `[X]` for unordered lists by peeking at
    // the upcoming token sequence; if present, consume it and emit a
    // `TaskListItem`. Otherwise fall through to a plain `ListItem`.
    if !ordered {
        let pre = p.pos;
        // Optional leading whitespace token between marker and `[`.
        if matches!(p.peek_kind(), Some(TokenKind::Whitespace)) {
            p.advance();
        }
        if matches!(p.peek_kind(), Some(TokenKind::Bracket)) {
            p.advance();
            let text_raw = p.peek().map(|t| t.raw.clone()).unwrap_or_default();
            let kind = p.peek_kind().cloned();
            if matches!(kind, Some(TokenKind::Text))
                && (text_raw == " " || text_raw.eq_ignore_ascii_case("x"))
            {
                p.advance();
                if matches!(p.peek_kind(), Some(TokenKind::Bracket)) {
                    p.advance();
                    let checked = text_raw.eq_ignore_ascii_case("x");
                    let inline = crate::inline::collect_inline_for_list_item(p);
                    return Node::TaskListItem(TaskListItem {
                        checked,
                        children: inline,
                        span: default_span(),
                    });
                }
            }
            // not a task list — roll back
            p.pos = pre;
        } else {
            p.pos = pre;
        }
    }

    let inline = crate::inline::collect_inline_for_list_item(p);
    Node::ListItem(ListItem {
        children: inline,
        span: default_span(),
    })
}

fn parse_blockquote(p: &mut Parser) -> Node {
    let mut paras: Vec<Node> = Vec::new();
    loop {
        if !matches!(p.peek_kind(), Some(TokenKind::BlockQuote)) {
            break;
        }
        p.advance();
        let inline = collect_inline_until_break(p);
        if !inline.is_empty() {
            paras.push(Node::Paragraph(Paragraph {
                children: inline,
                span: default_span(),
            }));
        }
        // consume one separator break
        if matches!(
            p.peek_kind(),
            Some(TokenKind::SoftBreak) | Some(TokenKind::HardBreak)
        ) {
            p.advance();
        }
    }
    Node::Blockquote(Blockquote {
        children: paras,
        span: default_span(),
    })
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
    // Setext heading: a paragraph followed by a SoftBreak and then a line of
    // pure `=` (level 1) or `-` (level 2) characters. The lexer emits each
    // `=` as an Eq token (raw == "="), and a run of `-` is emitted as a
    // single `ThematicBreak` token (raw is the dashes).
    if matches!(p.peek_kind(), Some(TokenKind::SoftBreak)) {
        let saved = p.pos;
        p.advance();
        if let Some(lvl) = peek_setext_underline(p) {
            consume_setext_underline(p);
            let plain = plain_text(&children);
            let id = slug::slugify(plain);
            return Node::Heading(Heading {
                level: lvl,
                id,
                children,
                span: default_span(),
            });
        }
        p.pos = saved;
    }
    Node::Paragraph(Paragraph {
        children,
        span: default_span(),
    })
}

/// Return Some(1) for an `=` underline, Some(2) for `-` underline, else None.
/// The position is left untouched.
fn peek_setext_underline(p: &Parser) -> Option<u8> {
    let t = p.tokens.get(p.pos)?;
    match &t.kind {
        TokenKind::Eq => {
            // run of consecutive Eq tokens, then SoftBreak/HardBreak/Eof
            let mut i = p.pos;
            while let Some(tt) = p.tokens.get(i) {
                if matches!(tt.kind, TokenKind::Eq) {
                    i += 1;
                } else {
                    break;
                }
            }
            let next = p.tokens.get(i).map(|t| &t.kind);
            if matches!(
                next,
                Some(TokenKind::SoftBreak) | Some(TokenKind::HardBreak) | Some(TokenKind::Eof) | None
            ) {
                Some(1)
            } else {
                None
            }
        }
        TokenKind::ThematicBreak => {
            // `--` (2+ dashes) on its own line ends up as ThematicBreak with
            // raw == "---..." (all dashes). Only treat as setext h2 when raw
            // is purely `-` characters.
            if !t.raw.is_empty() && t.raw.chars().all(|c| c == '-') {
                Some(2)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn consume_setext_underline(p: &mut Parser) {
    if let Some(t) = p.tokens.get(p.pos) {
        match t.kind {
            TokenKind::Eq => {
                while matches!(p.peek_kind(), Some(TokenKind::Eq)) {
                    p.advance();
                }
            }
            TokenKind::ThematicBreak => {
                p.advance();
            }
            _ => {}
        }
    }
}

fn parse_code_block(p: &mut Parser) -> Node {
    let fence_n = match p.peek_kind() {
        Some(TokenKind::CodeStart(n)) => *n,
        _ => 3,
    };
    p.advance(); // CodeStart

    // Info string: a single Text token (may be empty).
    let info = match p.peek() {
        Some(t) if matches!(t.kind, TokenKind::Text) => {
            let raw = t.raw.clone();
            p.advance();
            raw
        }
        _ => String::new(),
    };
    let info_trimmed = info.trim();
    let (lang, meta) = if info_trimmed.is_empty() {
        (None, None)
    } else {
        match info_trimmed.split_once(char::is_whitespace) {
            Some((l, rest)) => {
                let rest = rest.trim();
                (
                    Some(l.to_string()),
                    if rest.is_empty() {
                        None
                    } else {
                        Some(rest.to_string())
                    },
                )
            }
            None => (Some(info_trimmed.to_string()), None),
        }
    };

    // Body: concat all Text tokens until matching CodeEnd(n).
    let mut value = String::new();
    while let Some(t) = p.peek() {
        match &t.kind {
            TokenKind::CodeEnd(m) if *m == fence_n => {
                p.advance();
                break;
            }
            TokenKind::Eof => break,
            TokenKind::Text => {
                value.push_str(&t.raw);
                p.advance();
            }
            _ => {
                value.push_str(&t.raw);
                p.advance();
            }
        }
    }

    Node::CodeBlock(CodeBlock {
        lang,
        meta,
        value,
        raw: None,
        commands: None,
        highlighted_html: None,
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
