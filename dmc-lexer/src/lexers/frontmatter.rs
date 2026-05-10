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
