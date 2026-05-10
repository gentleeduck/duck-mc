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

    // Skip whitespace after `:`. CM 4.7 allows the destination to
    // start on the next line (after exactly one newline + optional
    // leading whitespace).
    let mut j = i + 2;
    while j < bytes.len() && matches!(bytes[j], b' ' | b'\t') {
      j += 1;
    }
    if j < bytes.len() && bytes[j] == b'\n' {
      // Look at the next line: blank line means malformed def.
      j += 1;
      while j < bytes.len() && matches!(bytes[j], b' ' | b'\t') {
        j += 1;
      }
      if j >= bytes.len() || bytes[j] == b'\n' {
        return false;
      }
    }
    if j >= bytes.len() {
      return false;
    }

    // Destination: `<bracketed>` or bare run.
    if bytes[j] == b'<' {
      let mut p = j + 1;
      while p < bytes.len() && bytes[p] != b'>' && bytes[p] != b'\n' {
        p += 1;
      }
      if p >= bytes.len() || bytes[p] != b'>' {
        return false;
      }
      j = p + 1;
    } else {
      let dest_start = j;
      while j < bytes.len() && !matches!(bytes[j], b' ' | b'\t' | b'\n') {
        j += 1;
      }
      if dest_start == j {
        return false;
      }
    }

    // Optional title. Can be on the same line (after whitespace) or
    // on the next line (after exactly one newline + optional indent).
    let title_search_start = j;
    while j < bytes.len() && matches!(bytes[j], b' ' | b'\t') {
      j += 1;
    }
    let cross_newline = if j < bytes.len() && bytes[j] == b'\n' {
      let after_nl = j + 1;
      let mut k = after_nl;
      while k < bytes.len() && matches!(bytes[k], b' ' | b'\t') {
        k += 1;
      }
      // Blank line ends the def -- no title.
      if k >= bytes.len() || bytes[k] == b'\n' {
        // Definition ends at the original newline; leave `j` here.
        false
      } else {
        j = k;
        true
      }
    } else {
      false
    };
    if j < bytes.len() && matches!(bytes[j], b'"' | b'\'' | b'(') {
      let open = bytes[j];
      let close = if open == b'(' { b')' } else { open };
      let mut p = j + 1;
      let mut found = false;
      while p < bytes.len() {
        if bytes[p] == b'\n' && p + 1 < bytes.len() && bytes[p + 1] == b'\n' {
          break; // Blank line within title aborts.
        }
        if bytes[p] == close {
          found = true;
          p += 1;
          break;
        }
        if bytes[p] == b'\\' && p + 1 < bytes.len() {
          p += 2;
          continue;
        }
        p += 1;
      }
      if found {
        j = p;
      } else if cross_newline {
        // No matching close on the title line; fall back to no-title
        // definition that ends at the original line break.
        j = title_search_start;
      } else {
        // Single-line title with no closer -- consume rest of line.
        while j < bytes.len() && bytes[j] != b'\n' {
          j += 1;
        }
      }
    } else if cross_newline {
      // No title on next line; def ends at the prior newline.
      j = title_search_start;
    } else {
      // Bare destination, eat trailing whitespace on the same line.
      while j < bytes.len() && bytes[j] != b'\n' {
        j += 1;
      }
    }

    // CM 4.7: nothing other than whitespace may appear after the title
    // (or the destination, if no title) on the same line. If a title
    // *was* matched but has trailing junk, fall back to the no-title
    // form (rewind to before the title) so the def keeps just the
    // destination. If even the no-title form has junk after the dest,
    // reject so the surrounding line falls through as a paragraph.
    let check_tail = |from: usize| -> bool {
      let mut t = from;
      while t < bytes.len() && matches!(bytes[t], b' ' | b'\t') {
        t += 1;
      }
      t >= bytes.len() || bytes[t] == b'\n'
    };
    if !check_tail(j) {
      if j != title_search_start && check_tail(title_search_start) {
        j = title_search_start;
      } else {
        return false;
      }
    }

    self.advance_bytes(j - self.current);
    self.emit(TokenKind::LinkRefDef);
    true
  }
}
