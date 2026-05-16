use crate::ast::*;
use crate::parser::Parser;
use dmc_lexer::token::TokenKind;

impl<'eng, 'tokens> Parser<'eng, 'tokens> {
  /// Reconstruct the upcoming logical line as `(text, token_count)`. Stops
  /// at the first break, EOF, or block-level boundary token.
  fn collect_line_text(&self) -> Option<(String, usize)> {
    let mut text = String::new();
    let mut count = 0usize;
    while let Some(t) = self.tokens.get(self.pos + count) {
      match &t.kind {
        TokenKind::SoftBreak
        | TokenKind::HardBreak
        | TokenKind::BlankLine
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

  /// Speculatively parse a GFM table. Rolls back `pos` on mismatch so the
  /// caller can fall through to `parse_paragraph`.
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
    // GFM: separator column count must match header column count.
    let header_cells_preview = split_cells(&line1);
    if header_cells_preview.len() != aligns.len() {
      self.pos = saved;
      return None;
    }
    self.pos += len2;
    if matches!(self.peek_kind(), Some(TokenKind::SoftBreak) | Some(TokenKind::HardBreak)) {
      self.advance();
    }

    let header_cells = split_cells(&line1);
    let mut rows = Vec::new();
    rows.push(make_row(&header_cells, &span));

    let col_count = aligns.len();
    while let Some((line, len)) = self.collect_line_text() {
      let trimmed = line.trim();
      if trimmed.is_empty() {
        break;
      }
      // GFM 4.10: subsequent rows can omit pipes; block constructs break out.
      if line_starts_block_construct(trimmed) {
        break;
      }
      let mut cells = split_cells(&line);
      // GFM 4.10: pad short rows, truncate long ones.
      if cells.len() < col_count {
        cells.resize(col_count, String::new());
      } else if cells.len() > col_count {
        cells.truncate(col_count);
      }
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

/// GFM allows optional leading/trailing `|`; accept any line with at least
/// one `|`. Pure-separator lines are caught by `parse_alignment_row`.
fn looks_like_table_row(s: &str) -> bool {
  let t = s.trim();
  if t.is_empty() {
    return false;
  }
  t.matches('|').count() >= 1
}

/// Lines that break out of an open table even without `|`: block openers
/// (heading, fence, list, bq, thematic break).
fn line_starts_block_construct(t: &str) -> bool {
  if t.starts_with('#') || t.starts_with('>') || t.starts_with("```") || t.starts_with("~~~") {
    return true;
  }
  // Thematic break: 3+ of `-`/`*`/`_`, optionally ws-separated.
  let bytes = t.as_bytes();
  if !bytes.is_empty() && matches!(bytes[0], b'-' | b'*' | b'_') {
    let marker = bytes[0];
    let mut count = 0usize;
    let ok = bytes.iter().all(|&b| match b {
      b' ' | b'\t' => true,
      b if b == marker => {
        count += 1;
        true
      },
      _ => false,
    });
    if ok && count >= 3 {
      return true;
    }
  }
  if t.len() >= 2 && matches!(bytes[0], b'-' | b'*' | b'+') && matches!(bytes.get(1).copied(), Some(b' ') | Some(b'\t'))
  {
    return true;
  }
  if bytes[0].is_ascii_digit() {
    let digits = bytes.iter().take_while(|b| b.is_ascii_digit()).count();
    if digits > 0
      && digits < bytes.len()
      && matches!(bytes[digits], b'.' | b')')
      && matches!(bytes.get(digits + 1).copied(), Some(b' ') | Some(b'\t'))
    {
      return true;
    }
  }
  false
}

/// Parse `|:---|---:|:---:|`. Leading/trailing `|` optional (GFM 4.10).
fn parse_alignment_row(s: &str) -> Option<Vec<TableAlign>> {
  let t = s.trim();
  let inner = t.strip_prefix('|').unwrap_or(t);
  let inner = inner.strip_suffix('|').unwrap_or(inner);
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

/// Split `|a|b|c|` into cell strings. Pipes inside `` `code` `` spans and
/// `\|` escapes are content, not delimiters - required for GFM tables like
/// `` | `"x" \| "y"` | `"z"` |``.
fn split_cells(s: &str) -> Vec<String> {
  let t = s.trim();
  if t.is_empty() {
    return Vec::new();
  }
  let inner = t.strip_prefix('|').unwrap_or(t);
  let inner = inner.strip_suffix('|').unwrap_or(inner);

  let mut cells: Vec<String> = Vec::new();
  let mut current = String::new();
  let mut chars = inner.chars().peekable();
  let mut in_code = false;
  while let Some(c) = chars.next() {
    match c {
      '\\' => {
        // GFM: `\|` is a literal pipe; forward without the backslash so the
        // inline parser sees the intended content.
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

/// Build a `TableRow` from raw cell strings. Each cell is re-lexed +
/// inline-parsed so inline markdown renders the same as in paragraphs.
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
