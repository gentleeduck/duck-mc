//! Top-level dispatch. Given the just-consumed char, route to the right
//! sub-lexer. Helpers here only inspect cursor context; actual emission
//! lives in `crate::lexers`.

use crate::{
  Lexer,
  token::{AutolinkKind, TokenKind},
};

impl<'eng, 'src: 'eng> Lexer<'eng, 'src> {
  /// Dispatch the just-consumed `c` to the matching sub-lexer.
  pub(crate) fn lex_tokens(&mut self, c: char) {
    match c {
      // Trivia
      '\n' => self.lex_newline(),
      ' ' | '\t' => self.lex_whitespace(),
      '\\' if self.peek() == Some('\n') => self.lex_newline(),
      '\\' if self.peek().is_some_and(Self::is_escapable_punct) => {
        self.advance();
        self.lex_text();
      },

      // ESM at column 0
      'i' if self.start_column == 0 && self.lexeme_starts_with("import") => {
        if !self.try_lex_esm("import") {
          self.lex_text();
        }
      },
      'e' if self.start_column == 0 && self.lexeme_starts_with("export") => {
        if !self.try_lex_esm("export") {
          self.lex_text();
        }
      },

      // Headings
      '#' if self.start_column == 0 => self.lex_heading(),
      '#' => {
        if !self.lex_heading_trailing_hashes() {
          self.lex_text();
        }
      },

      // Block quotes
      '>' if self.start_column == 0 => self.lex_block_quote(),

      // Ordered list marker
      '0'..='9' if self.start_column == 0 => {
        if !self.try_lex_ordered_list_marker() {
          self.lex_text();
        }
      },

      // Setext underline
      '=' if self.start_column == 0 => {
        if !self.try_lex_setext_underline() {
          self.lex_text();
        }
      },

      // Code fences and inline code
      '`' if self.start_column == 0 => {
        if self.try_lex_fenced_code('`') {
        } else if self.try_lex_inline_code() {
        } else {
          self.lex_text();
        }
      },
      '`' => {
        if !self.try_lex_inline_code() {
          self.skip_while_byte(b'`');
          self.emit(TokenKind::Text);
        }
      },

      // Tilde fence and strikethrough
      '~' if self.start_column == 0 => {
        if self.try_lex_fenced_code('~') {
        } else if self.try_lex_strikethrough() {
        } else {
          self.lex_text();
        }
      },
      '~' => {
        if !self.try_lex_strikethrough() {
          self.lex_text();
        }
      },

      // Entity references
      '&' => {
        if !self.try_lex_entity() {
          self.emit(TokenKind::Text);
        }
      },

      // MDX expressions and comments
      '{' if self.peek() == Some('/') && self.peek_next() == Some('*') => self.lex_mdx_comment(),
      '{' => self.lex_mdx_expression(),

      // Bare autolinks (GFM)
      'w' if self.lexeme_starts_with("www.") && self.try_lex_bare_autolink(AutolinkKind::BareWww) => {},
      'h' if self.lexeme_starts_with("http") && self.try_lex_bare_autolink(AutolinkKind::BareUrl) => {},

      '<' if self.starts_cdata() => {
        if !self.try_lex_cdata() {
          self.lex_text();
        }
      },
      '<' if self.starts_declaration() => {
        if !self.try_lex_declaration() {
          self.lex_text();
        }
      },
      '<' if self.peek() == Some('?') => {
        if !self.try_lex_processing_instruction() {
          self.lex_text();
        }
      },

      // Angle constructs: autolink, HTML comment, JSX tag, or text
      '<' if self.is_angle_autolink() => self.lex_angle_autolink(),
      '<' if self.starts_html_comment() => {
        if !self.try_lex_html_comment() {
          self.lex_text();
        }
      },
      '<' if self.peek_starts_jsx() => {
        if !self.try_lex_jsx_tag() {
          self.lex_text();
        }
      },
      '<' => self.lex_text(),

      // Brackets, parens, image marker
      '[' => {
        if self.try_lex_task_marker() {
        } else if self.try_lex_footnote() {
        } else if self.start_column == 0 && self.try_lex_link_ref_def() {
        } else {
          self.emit(TokenKind::LinkOpen);
        }
      },
      ']' => self.emit(TokenKind::LinkClose),
      '(' => self.emit(TokenKind::LinkTargetOpen),
      ')' => self.emit(TokenKind::LinkTargetClose),
      '!' if self.peek() == Some('[') => {
        self.advance();
        self.emit(TokenKind::ImageMarker);
      },

      // Tables: parser disambiguates rows
      '|' => self.emit(TokenKind::TablePipe),

      // Thematic break / list marker / emphasis (column-0 cascade)
      '-' | '*' | '_' if self.start_column == 0 => {
        if self.lex_thematic_break(c) {
        } else if c != '_' && self.lex_unordered_list_marker() {
        } else if c == '*' || c == '_' {
          self.lex_emphasis(c);
        } else {
          self.lex_text();
        }
      },
      '*' | '_' => self.lex_emphasis(c),

      '+' if self.start_column == 0 => {
        if !self.lex_unordered_list_marker() {
          self.lex_text();
        }
      },

      _ => self.lex_text(),
    }
  }

  /// CM-escapable punctuation set excluding `\n` (handled as line break)
  /// and `\\` itself (handled by `lex_text`'s pair arm).
  #[inline]
  pub(crate) fn is_escapable_punct(c: char) -> bool {
    matches!(c, '*' | '_' | '`' | '<' | '>' | '{' | '}' | '[' | ']' | '(' | ')' | '!' | '#' | '-' | '|' | '~')
  }

  /// Whether the next char looks like the start of a JSX tag.
  pub(crate) fn peek_starts_jsx(&self) -> bool {
    match self.peek() {
      Some('>') => true,
      Some('/') => match self.peek_next() {
        Some('>') => true,
        Some(c) if c.is_ascii_alphabetic() => true,
        _ => false,
      },
      Some(c) if c.is_ascii_alphabetic() => true,
      _ => false,
    }
  }

  /// Whether the cursor (already past `<`) sits at `!--`.
  pub(crate) fn starts_html_comment(&self) -> bool {
    let b = self.source.as_bytes();
    self.peek() == Some('!') && b.get(self.current + 1) == Some(&b'-') && b.get(self.current + 2) == Some(&b'-')
  }

  /// Lookahead test for `<...>` autolinks. True when the upcoming `>`
  /// closes either a URL (`<https://...>`) or an email (`<a@b.c>`).
  pub(crate) fn is_angle_autolink(&self) -> bool {
    let rest = match self.source.get(self.current..) {
      Some(s) => s,
      None => return false,
    };
    let bytes = rest.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
      if b == b'>' {
        let inner = &rest[..i];
        // URL form: valid scheme + `:` + non-empty body.
        if let Some(colon) = inner.find(':') {
          let scheme = &inner[..colon];
          if Self::is_uri_scheme(scheme) && colon + 1 < inner.len() {
            return true;
          }
        }
        // Email form.
        if let Some(at) = inner.find('@') {
          let (local, domain) = inner.split_at(at);
          let domain = &domain[1..];
          if !local.is_empty()
            && domain.contains('.')
            && !domain.starts_with('.')
            && !domain.ends_with('.')
            && local.chars().all(|c| c.is_ascii_alphanumeric() || ".+-_".contains(c))
            && domain.chars().all(|c| c.is_ascii_alphanumeric() || ".-".contains(c))
          {
            return true;
          }
        }
        return false;
      }
      if matches!(b, b' ' | b'\n' | b'\t' | b'<') {
        return false;
      }
    }
    false
  }

  /// CM 6.5: scheme = ASCII letter, then 1-31 of `[A-Za-z0-9+.-]`.
  /// Total length 2-32.
  pub(crate) fn is_uri_scheme(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() < 2 || b.len() > 32 || !b[0].is_ascii_alphabetic() {
      return false;
    }
    b[1..].iter().all(|&c| c.is_ascii_alphanumeric() || c == b'+' || c == b'.' || c == b'-')
  }

  pub(crate) fn starts_cdata(&self) -> bool {
    let b = self.source.as_bytes();
    let i = self.current;
    i + 7 < b.len() && &b[i..i + 8] == b"![CDATA["
  }

  pub(crate) fn starts_declaration(&self) -> bool {
    let b = self.source.as_bytes();
    b.get(self.current) == Some(&b'!') && matches!(b.get(self.current + 1), Some(c) if c.is_ascii_uppercase())
  }
}
