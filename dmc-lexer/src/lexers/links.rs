//! Link-related and angle-bracket constructs: bare/angle autolinks, HTML
//! comments, footnotes, and link reference definitions.

use crate::{
  Lexer,
  token::{AutolinkKind, TokenKind},
};

impl<'eng, 'src: 'eng> Lexer<'eng, 'src> {
  /// Consume `<url>` or `<email>`. Caller has already advanced past the
  /// opening `<` and `is_angle_autolink` returned true.
  pub(crate) fn lex_angle_autolink(&mut self) {
    let bytes = self.source.as_bytes();
    let mut i = self.current;
    while i < bytes.len() && bytes[i] != b'>' {
      i += 1;
    }
    let inner = &self.source[self.current..i];
    // CM 6.5: classify by scheme. If the part before the first `:` is
    // a valid URI scheme (alpha-lead, 2-32 chars, `[A-Za-z0-9+.-]`),
    // treat as URL. Otherwise an `@` makes it an email.
    let kind = match inner.find(':') {
      Some(colon) if Self::is_uri_scheme(&inner[..colon]) => AutolinkKind::AngleUrl,
      _ if inner.contains('@') => AutolinkKind::AngleEmail,
      _ => AutolinkKind::AngleUrl,
    };
    let total = i + 1 - self.current;
    self.advance_bytes(total);
    self.emit(TokenKind::Autolink(kind));
  }

  /// GFM bare autolink. Caller is on the `h` of `http(s)://...` or the
  /// `w` of `www.x.y`. The first char is already consumed.
  pub(crate) fn try_lex_bare_autolink(&mut self, kind: AutolinkKind) -> bool {
    let bytes = self.source.as_bytes();

    // Validate prefix and find body start.
    let body_start = match kind {
      AutolinkKind::BareUrl => {
        if self.source[self.start..].starts_with("https://") {
          self.start + 8
        } else if self.source[self.start..].starts_with("http://") {
          self.start + 7
        } else {
          return false;
        }
      },
      AutolinkKind::BareWww => {
        if !self.source[self.start..].starts_with("www.") {
          return false;
        }
        self.start + 4
      },
      _ => return false,
    };

    // Consume URL chars until whitespace or angle bracket.
    let mut i = body_start;
    while i < bytes.len() {
      match bytes[i] {
        b' ' | b'\t' | b'\n' | b'\r' | b'<' | b'>' => break,
        _ => i += 1,
      }
    }

    // GFM trims trailing punctuation `?!.,:*_~` and unbalanced `)`.
    while i > body_start {
      let c = bytes[i - 1];
      if matches!(c, b'?' | b'!' | b'.' | b',' | b':' | b'*' | b'_' | b'~') {
        i -= 1;
        continue;
      }
      if c == b')' {
        let opens = bytes[body_start..i].iter().filter(|&&b| b == b'(').count();
        let closes = bytes[body_start..i].iter().filter(|&&b| b == b')').count();
        if closes > opens {
          i -= 1;
          continue;
        }
      }
      break;
    }

    // Sanity check: BareWww needs a `.` after `www.`; BareUrl needs body.
    let body = &bytes[body_start..i];
    match kind {
      AutolinkKind::BareWww if !body.contains(&b'.') => return false,
      AutolinkKind::BareUrl if body.is_empty() => return false,
      _ => {},
    }

    self.advance_bytes(i - self.current);
    self.emit(TokenKind::Autolink(kind));
    true
  }

  /// HTML comment `<!-- ... -->`. The opening `<` is already consumed.
  /// Unclosed comments run to EOF and emit body as Text without a closer.
  pub(crate) fn try_lex_html_comment(&mut self) -> bool {
    let bytes = self.source.as_bytes();
    if bytes.get(self.current) != Some(&b'!')
      || bytes.get(self.current + 1) != Some(&b'-')
      || bytes.get(self.current + 2) != Some(&b'-')
    {
      return false;
    }

    self.advance();
    self.advance();
    self.advance();
    self.emit(TokenKind::HtmlCommentOpen);

    loop {
      match self.peek() {
        None => {
          if self.current > self.start {
            self.emit(TokenKind::Text);
          }
          return true;
        },
        Some('-') => {
          let b = self.source.as_bytes();
          if b.get(self.current + 1) == Some(&b'-') && b.get(self.current + 2) == Some(&b'>') {
            if self.current > self.start {
              self.emit(TokenKind::Text);
            }
            self.advance();
            self.advance();
            self.advance();
            self.emit(TokenKind::HtmlCommentClose);
            return true;
          }
          self.advance();
        },
        _ => {
          self.advance();
        },
      }
    }
  }

  /// Footnote reference `[^id]` (inline) or definition `[^id]:` (col 0).
  /// Caller has already consumed `[`.
  pub(crate) fn try_lex_footnote(&mut self) -> bool {
    if self.peek() != Some('^') {
      return false;
    }

    let bytes = self.source.as_bytes();
    let mut i = self.current + 1;
    while i < bytes.len() {
      match bytes[i] {
        b']' => break,
        b'\n' | b' ' | b'\t' | b'[' => return false,
        _ => i += 1,
      }
    }
    if i >= bytes.len() || bytes[i] != b']' || i == self.current + 1 {
      return false;
    }

    let is_def = self.start_column == 0 && bytes.get(i + 1) == Some(&b':');

    if is_def {
      let end = i + 2;
      self.advance_bytes(end - self.current);
      if matches!(self.peek(), Some(' ' | '\t')) {
        self.advance();
      }
      self.emit(TokenKind::FootnoteDefMarker);
      true
    } else {
      let end = i + 1;
      self.advance_bytes(end - self.current);
      self.emit(TokenKind::FootnoteRefOpen);
      true
    }
  }

  /// CM 4.7 link reference definition: `[label]: url "optional title"`
  /// at column 0. The opening `[` is already consumed.
  pub(crate) fn try_lex_link_ref_def(&mut self) -> bool {
    let bytes = self.source.as_bytes();
    let mut i = self.current;
    while i < bytes.len() {
      match bytes[i] {
        b']' => break,
        b'\n' | b'[' => return false,
        b'\\' if i + 1 < bytes.len() => i += 2,
        _ => i += 1,
      }
    }
    if i >= bytes.len() || bytes[i] != b']' || i == self.current {
      return false;
    }
    if bytes.get(i + 1) != Some(&b':') {
      return false;
    }

    // Skip whitespace after `:`.
    let mut j = i + 2;
    while j < bytes.len() && matches!(bytes[j], b' ' | b'\t') {
      j += 1;
    }
    if j >= bytes.len() || matches!(bytes[j], b'\n') {
      return false;
    }

    // Consume URL until whitespace.
    while j < bytes.len() && !matches!(bytes[j], b' ' | b'\t' | b'\n') {
      j += 1;
    }
    // Consume optional title (rest of line).
    while j < bytes.len() && bytes[j] != b'\n' {
      j += 1;
    }

    self.advance_bytes(j - self.current);
    self.emit(TokenKind::LinkRefDef);
    true
  }
}
