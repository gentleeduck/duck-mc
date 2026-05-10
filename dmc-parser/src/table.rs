use crate::ast::*;
use crate::parser::Parser;
use dmc_lexer::token::TokenKind;

impl<'eng, 'tokens> Parser<'eng, 'tokens> {
  /// Reconstruct the upcoming logical line from tokens, stopping at the first
  /// break, EOF, or block-level boundary token. Returns `(text, token_count)`.
  fn collect_line_text(&self) -> Option<(String, usize)> {
    let mut text = String::new();
    let mut count = 0usize;
    while let Some(t) = self.tokens.get(self.pos + count) {
      match &t.kind {
        TokenKind::SoftBreak
        | TokenKind::HardBreak
        | TokenKind::Eof
        | TokenKind::Heading(_)
        | TokenKind::FrontmatterStart(_)
        | TokenKind::Import
        | TokenKind::Export => break,
        _ => {
          text.push_str(t.raw);
          count += 1;
        },
      }
    }
    if count == 0 { None } else { Some((text, count)) }
  }

  /// Speculatively parse a GFM table at the cursor. Rolls back `pos` on any
  /// mismatch so the caller can fall through to `parse_paragraph`.
  pub(crate) fn try_parse_table(&mut self) -> Option<Node> {
    let saved = self.pos;
    let span = self.current_span();
    let (line1, len1) = self.collect_line_text()?;
    if !looks_like_table_row(&line1) {
      return None;
    }
    self.pos += len1;
    if matches!(self.peek_kind(), Some(TokenKind::SoftBreak) | Some(TokenKind::HardBreak)) {
      self.advance();
    }
    let (line2, len2) = match self.collect_line_text() {
      Some(x) => x,
      None => {
        self.pos = saved;
        return None;
      },
    };
    let aligns = match parse_alignment_row(&line2) {
      Some(a) => a,
      None => {
        self.pos = saved;
        return None;
      },
    };
    self.pos += len2;
    if matches!(self.peek_kind(), Some(TokenKind::SoftBreak) | Some(TokenKind::HardBreak)) {
      self.advance();
    }

    let header_cells = split_cells(&line1);
    let mut rows = Vec::new();
    rows.push(make_row(&header_cells, &span));

    while let Some((line, len)) = self.collect_line_text() {
      if !looks_like_table_row(&line) {
        break;
      }
      let cells = split_cells(&line);
      let row_span = self.current_span();
      rows.push(make_row(&cells, &row_span));
      self.pos += len;
      if matches!(self.peek_kind(), Some(TokenKind::SoftBreak) | Some(TokenKind::HardBreak)) {
        self.advance();
      }
    }

    Some(Node::Table(Table { align: aligns, children: rows, span }))
  }
}

/// True when `s` starts and ends with `|` and has at least two pipes.
fn looks_like_table_row(s: &str) -> bool {
  let t = s.trim();
  t.starts_with('|') && t.ends_with('|') && t.matches('|').count() >= 2
}

/// Parse the `|:---|---:|:---:|` alignment row. `None` on any malformed cell.
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

/// Split `|a|b|c|` into the cell strings between pipes (no trim; caller
/// trims when materialising the cell).
///
/// Pipes inside an inline-code span (`` ` `` … `` ` ``) and pipes
/// escaped with `\|` are *content*, not delimiters. GFM's table grammar
/// requires this; without it, a row like
/// `` | `"single" \| "multiple"` | `"single"` | `` is mis-split into
/// three cells. Track the escape and code-span state while walking the
/// row.
fn split_cells(s: &str) -> Vec<String> {
  let t = s.trim();
  if t.len() < 2 {
    return Vec::new();
  }
  let inner = &t[1..t.len() - 1];

  let mut cells: Vec<String> = Vec::new();
  let mut current = String::new();
  let mut chars = inner.chars().peekable();
  let mut in_code = false;
  while let Some(c) = chars.next() {
    match c {
      '\\' => {
        // GFM: `\|` inside a table cell is a literal pipe, not a delimiter.
        // Forward the escaped character verbatim (without the backslash) so
        // the inline parser sees the intended content.
        if let Some(&next) = chars.peek() {
          if next == '|' {
            chars.next();
            current.push('|');
          } else {
            current.push('\\');
          }
        } else {
          current.push('\\');
        }
      },
      '`' => {
        in_code = !in_code;
        current.push('`');
      },
      '|' if !in_code => {
        cells.push(std::mem::take(&mut current));
      },
      _ => current.push(c),
    }
  }
  cells.push(current);
  cells
}

/// Build one `TableRow` from raw cell strings. Each cell string is
/// re-lexed and inline-parsed, so backticks, bold, italic, links, and
/// other inline markdown render the same as in paragraphs.
fn make_row(cells: &[String], span: &duck_diagnostic::Span) -> TableRow {
  TableRow {
    cells: cells
      .iter()
      .map(|c| {
        let trimmed = c.trim();
        let children = if trimmed.is_empty() {
          Vec::new()
        } else {
          let mut nodes = crate::parser::parse_inline_str(trimmed);
          if nodes.is_empty() {
            nodes.push(Node::Text(Text { value: trimmed.to_string(), span: span.clone() }));
          }
          nodes
        };
        TableCell { children, span: span.clone() }
      })
      .collect(),
    span: span.clone(),
  }
}
