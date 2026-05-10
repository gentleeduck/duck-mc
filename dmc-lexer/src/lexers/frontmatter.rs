//! Frontmatter blocks: `---` (YAML) or `+++` (TOML) or `{` JSON at file
//! start, body, and matching close fence. Called once at the start of
//! `scan_tokens`.

use crate::{
  Lexer,
  token::{FrontmatterKind, TokenKind},
};

impl<'eng, 'src: 'eng> Lexer<'eng, 'src> {
  /// Try lexing frontmatter at the start of the file. Emits nothing if the
  /// source doesn't start with a recognized fence + matching closer.
  pub(crate) fn try_lex_frontmatter(&mut self) {
    let bytes = self.source.as_bytes();
    let kind = match bytes {
      [b'-', b'-', b'-', b'\n', ..] => FrontmatterKind::Yaml,
      [b'+', b'+', b'+', b'\n', ..] => FrontmatterKind::Toml,
      [b'{', ..] => return self.try_lex_json_frontmatter(),
      _ => return,
    };
    let fence: &[u8; 3] = match kind {
      FrontmatterKind::Yaml => b"---",
      FrontmatterKind::Toml => b"+++",
      FrontmatterKind::Json => return,
    };

    let after_open = 4;
    let needle = [b'\n', fence[0], fence[1], fence[2]];
    let rest = &bytes[after_open..];
    let Some(close_rel) = Self::find_subslice(rest, &needle) else { return };
    let close_abs = after_open + close_rel;

    let after_close = close_abs + 4;
    let valid_end = bytes.get(after_close).is_none_or(|&b| b == b'\n');
    if !valid_end {
      return;
    }

    // Heuristic: only commit to frontmatter when the body looks like
    // YAML/TOML rather than CM thematic-break interspersed text. Empty
    // body or one containing `:` (YAML key: value) / `=` (TOML
    // assignment) qualifies; everything else falls through so the
    // outer dispatch can emit a ThematicBreak.
    let body = &bytes[after_open..close_abs];
    let trimmed_empty = body.iter().all(|&b| matches!(b, b' ' | b'\t' | b'\n'));
    let has_pair_marker = body.iter().any(|&b| match kind {
      FrontmatterKind::Yaml => b == b':',
      FrontmatterKind::Toml => b == b'=',
      FrontmatterKind::Json => false,
    });
    if !trimmed_empty && !has_pair_marker {
      return;
    }

    for _ in 0..3 {
      self.advance();
    }
    self.emit(TokenKind::FrontmatterStart(kind));
    self.advance();
    self.start = self.current;
    self.start_line = self.line;
    self.start_column = self.column;

    while self.current < close_abs {
      self.advance();
    }
    self.emit(TokenKind::FrontmatterContent);

    self.advance();
    self.start = self.current;
    self.start_line = self.line;
    self.start_column = self.column;
    for _ in 0..3 {
      self.advance();
    }
    self.emit(TokenKind::FrontmatterEnd(kind));
  }

  fn try_lex_json_frontmatter(&mut self) {
    let bytes = self.source.as_bytes();
    let mut i = 0;
    let mut depth: i32 = 0;
    let mut in_string = false;
    while i < bytes.len() {
      let b = bytes[i];
      if in_string {
        if b == b'\\' && i + 1 < bytes.len() {
          i += 2;
          continue;
        }
        if b == b'"' {
          in_string = false;
        }
        i += 1;
        continue;
      }
      match b {
        b'"' => in_string = true,
        b'{' => depth += 1,
        b'}' => {
          depth -= 1;
          if depth == 0 {
            break;
          }
        },
        _ => {},
      }
      i += 1;
    }
    if depth != 0 || i >= bytes.len() {
      return;
    }
    let close_idx = i;
    if !matches!(bytes.get(close_idx + 1), None | Some(b'\n')) {
      return;
    }

    // JSON frontmatter must look like an object body: at least one
    // `"key":` pair. Plain MDX expressions like `{2 + 2}` or
    // `{`hello`}` have no JSON-shaped keys, so they fall through to
    // the dispatch's expression / text path.
    let body = &bytes[1..close_idx];
    let has_json_key = {
      let mut found = false;
      let mut idx = 0;
      while idx < body.len() {
        if body[idx] == b'"' {
          // Scan to closing quote.
          idx += 1;
          let mut closed = false;
          while idx < body.len() {
            if body[idx] == b'\\' && idx + 1 < body.len() {
              idx += 2;
              continue;
            }
            if body[idx] == b'"' {
              closed = true;
              idx += 1;
              break;
            }
            idx += 1;
          }
          if closed {
            // Skip whitespace after closing quote, look for `:`.
            while idx < body.len() && matches!(body[idx], b' ' | b'\t' | b'\n' | b'\r') {
              idx += 1;
            }
            if idx < body.len() && body[idx] == b':' {
              found = true;
              break;
            }
          }
        } else {
          idx += 1;
        }
      }
      found
    };
    if !has_json_key {
      return;
    }

    // Opening `{`.
    self.advance();
    self.emit(TokenKind::FrontmatterStart(FrontmatterKind::Json));

    // Body up to (but not including) the closing `}`.
    while self.current < close_idx {
      self.advance();
    }
    self.emit(TokenKind::FrontmatterContent);

    // Closing `}`.
    self.advance();
    self.emit(TokenKind::FrontmatterEnd(FrontmatterKind::Json));
  }

  fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
  }
}
