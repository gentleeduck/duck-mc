use crate::parser::Parser;
use duck_md_ast::*;
use duck_md_lexer::token::TokenKind;

/// Reconstruct the upcoming "logical line" of source from tokens, stopping at
/// the first hard/soft break, EOF, or block-level boundary token. Returns
/// `(text, num_tokens_consumed_to_reach_break)`.
fn collect_line_text(p: &Parser) -> Option<(String, usize)> {
    let mut text = String::new();
    let mut count = 0usize;
    while let Some(t) = p.tokens.get(p.pos + count) {
        match &t.kind {
            TokenKind::SoftBreak
            | TokenKind::HardBreak
            | TokenKind::Eof
            | TokenKind::Heading(_)
            | TokenKind::FrontmatterStart
            | TokenKind::Import
            | TokenKind::Export => break,
            _ => {
                text.push_str(&t.raw);
                count += 1;
            }
        }
    }
    if count == 0 {
        None
    } else {
        Some((text, count))
    }
}

fn looks_like_table_row(s: &str) -> bool {
    let t = s.trim();
    t.starts_with('|') && t.ends_with('|') && t.matches('|').count() >= 2
}

fn parse_alignment_row(s: &str) -> Option<Vec<TableAlign>> {
    let t = s.trim();
    if !t.starts_with('|') || !t.ends_with('|') {
        return None;
    }
    let inner = &t[1..t.len() - 1];
    let mut aligns = Vec::new();
    for cell in inner.split('|') {
        let cell = cell.trim();
        if cell.is_empty() {
            return None;
        }
        let starts_colon = cell.starts_with(':');
        let ends_colon = cell.ends_with(':');
        let mid = cell.trim_matches(':');
        if mid.is_empty() || !mid.chars().all(|c| c == '-') {
            return None;
        }
        aligns.push(match (starts_colon, ends_colon) {
            (true, true) => TableAlign::Center,
            (true, false) => TableAlign::Left,
            (false, true) => TableAlign::Right,
            _ => TableAlign::None,
        });
    }
    Some(aligns)
}

fn split_cells(s: &str) -> Vec<String> {
    let t = s.trim();
    if t.len() < 2 {
        return Vec::new();
    }
    let inner = &t[1..t.len() - 1];
    inner.split('|').map(|c| c.to_string()).collect()
}

fn make_row(cells: &[String]) -> TableRow {
    TableRow {
        cells: cells
            .iter()
            .map(|c| TableCell {
                children: vec![Node::Text(Text {
                    value: c.trim().to_string(),
                    span: default_span(),
                })],
                span: default_span(),
            })
            .collect(),
        span: default_span(),
    }
}

pub(crate) fn try_parse_table(p: &mut Parser) -> Option<Node> {
    let saved = p.pos;
    let (line1, len1) = collect_line_text(p)?;
    if !looks_like_table_row(&line1) {
        return None;
    }
    p.pos += len1;
    if matches!(
        p.peek_kind(),
        Some(TokenKind::SoftBreak) | Some(TokenKind::HardBreak)
    ) {
        p.advance();
    }
    let (line2, len2) = match collect_line_text(p) {
        Some(x) => x,
        None => {
            p.pos = saved;
            return None;
        }
    };
    let aligns = match parse_alignment_row(&line2) {
        Some(a) => a,
        None => {
            p.pos = saved;
            return None;
        }
    };
    p.pos += len2;
    if matches!(
        p.peek_kind(),
        Some(TokenKind::SoftBreak) | Some(TokenKind::HardBreak)
    ) {
        p.advance();
    }

    let header_cells = split_cells(&line1);
    let mut rows = Vec::new();
    rows.push(make_row(&header_cells));

    while let Some((line, len)) = collect_line_text(p) {
        if !looks_like_table_row(&line) {
            break;
        }
        let cells = split_cells(&line);
        rows.push(make_row(&cells));
        p.pos += len;
        if matches!(
            p.peek_kind(),
            Some(TokenKind::SoftBreak) | Some(TokenKind::HardBreak)
        ) {
            p.advance();
        }
    }

    Some(Node::Table(Table {
        align: aligns,
        children: rows,
        span: default_span(),
    }))
}
