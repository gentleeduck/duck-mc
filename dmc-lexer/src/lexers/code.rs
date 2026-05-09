use crate::{Lexer, token::TokenKind};

impl<'eng, 'src: 'eng> Lexer<'eng, 'src> {
  /// Entry for `` ` ``. Counts opening backticks and dispatches to the fenced
  /// (>=3) or inline (1-2) flavor.
  pub(crate) fn lex_code(&mut self) {
    self.skip_while_byte(b'`');
    let count = (self.current - self.start) as u8;

    if count >= 3 {
      self.lex_fenced_code(count);
    } else {
      self.lex_inline_code(count);
    }
  }

  /// Lex a `` ```...``` `` fence: `CodeStart` + info-string `Text` + body
  /// `Text` + `CodeEnd`.
  fn lex_fenced_code(&mut self, count: u8) {
    self.emit(TokenKind::CodeStart(count));

    self.skip_until_byte(b'\n');
    self.emit(TokenKind::Text);

    if self.peek() == Some('\n') {
      self.advance();
      self.line += 1;
      self.column = 0;
      self.start = self.current;
    }

    loop {
      if self.is_eof() {
        self.emit(TokenKind::Text);
        return;
      }

      if self.column == 0 && self.peek() == Some('`') {
        let content_end = self.current;
        self.skip_while_byte(b'`');
        let closing_count = (self.current - content_end) as u8;

        // Per CommonMark §4.5: a closing code fence "may be followed
        // only by spaces, which are ignored". Anything else (info
        // string text, language tag, meta) means we're staring at a
        // NEW fence open inside the current body, not the close.
        if closing_count == count {
          let bytes = self.source.as_bytes();
          let mut i = self.current;
          let mut close_is_clean = true;
          while i < bytes.len() && bytes[i] != b'\n' {
            if bytes[i] != b' ' && bytes[i] != b'\t' {
              close_is_clean = false;
              break;
            }
            i += 1;
          }
          if close_is_clean {
            let saved_current = self.current;
            self.current = content_end;
            self.emit(TokenKind::Text);

            self.start = content_end;
            self.current = saved_current;
            self.emit(TokenKind::CodeEnd(count));
            return;
          }
        }

        self.skip_until_byte(b'\n');
        if self.peek() == Some('\n') {
          self.advance();
          self.line += 1;
          self.column = 0;
        }
        continue;
      }

      self.skip_until_byte(b'\n');

      if self.peek() == Some('\n') {
        self.advance();
        self.line += 1;
        self.column = 0;
      }
    }
  }

  /// Lex inline backtick code (1-2 `` ` ``). Per CommonMark, a code
  /// span "begins with a backtick string and ends with a backtick
  /// string of equal length"; line endings inside are treated like
  /// spaces, so inline spans MAY cross newlines. Bails at a blank line
  /// or at a `` ``` `` line-start that looks like a fence open.
  fn lex_inline_code(&mut self, count: u8) {
    self.emit(TokenKind::CodeStart(count));

    let mut at_line_start = false;
    let mut prev_was_newline = false;
    loop {
      match self.peek() {
        None => break,
        Some('\n') => {
          if prev_was_newline {
            break;
          }
          self.advance();
          self.line += 1;
          self.column = 0;
          prev_was_newline = true;
          at_line_start = true;
        },
        Some('`') => {
          let run_start = self.current;
          self.skip_while_byte(b'`');
          let run_len = (self.current - run_start) as u8;
          if run_len == count {
            let saved_current = self.current;
            self.current = run_start;
            self.emit(TokenKind::Text);
            self.start = run_start;
            self.current = saved_current;
            return self.emit(TokenKind::CodeEnd(run_len));
          }
          if at_line_start && run_len >= 3 {
            self.current = run_start;
            self.column = 0;
            break;
          }
          prev_was_newline = false;
          at_line_start = false;
        },
        Some(_) => {
          self.advance();
          prev_was_newline = false;
          at_line_start = false;
        },
      }
    }

    self.emit(TokenKind::Text);
  }
}
