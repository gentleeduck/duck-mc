use duck_diagnostic::{Diagnostic, Label, Span};

use crate::{Lexer, token::TokenKind};
use dmc_diagnostic::Code;

impl<'eng, 'src: 'eng> Lexer<'eng, 'src> {
  /// Entry point for `` ` ``. Counts opening backticks and dispatches to the
  /// fenced (>=3) or inline (1-2) flavor.
  pub(crate) fn lex_code(&mut self) {
    // first backtick already consumed by caller - its column is one back
    let open_line = self.line;
    let open_col = self.column.saturating_sub(1);

    // consume all opening backticks (first already consumed by caller)
    self.skip_while_byte(b'`');
    let count = (self.current - self.start) as u8;

    if count >= 3 {
      self.lex_fenced_code(count, open_line, open_col);
    } else {
      self.lex_inline_code(count, open_line, open_col);
    }
  }

  /// Handle a `\`\`\`...\`\`\`` fence. Emits `CodeStart` + info-string `Text`
  /// + body `Text` + `CodeEnd`, or a diagnostic if no closing fence is found.
  fn lex_fenced_code(&mut self, count: u8, open_line: usize, open_col: usize) {
    self.emit(TokenKind::CodeStart(count));

    // consume the info string (e.g. "js showLineNumbers") until newline
    self.skip_until_byte(b'\n');
    self.emit(TokenKind::Text);

    // consume the newline after info string
    if self.peek() == Some('\n') {
      self.advance();
      self.line += 1;
      self.column = 0;
      self.start = self.current;
    }

    // consume content line by line until closing backticks at column 0
    loop {
      if self.is_eof() {
        self.emit(TokenKind::Text);
        self.emit_diagnostic(
          Diagnostic::new(Code::UnterminatedCodeBlock, "unterminated code block")
            .with_label(Label::primary(
              Span::from_zero_based("", open_line, open_col, count as usize),
              Some("fence opens here".to_string()),
            ))
            .with_label(Label::secondary(
              Span::from_zero_based("", self.line, self.column, 1),
              Some("end of file reached without a closing fence".to_string()),
            ))
            .with_help("add a closing ``` on its own line"),
        );
        return;
      }

      if self.column == 0 && self.peek() == Some('`') {
        let content_end = self.current;
        self.skip_while_byte(b'`');
        let closing_count = (self.current - content_end) as u8;

        if closing_count == count {
          // emit content before the closing backticks
          let saved_current = self.current;
          self.current = content_end;
          self.emit(TokenKind::Text);

          // emit closing backticks
          self.start = content_end;
          self.current = saved_current;
          self.emit(TokenKind::CodeEnd(count));
          return;
        }

        // not a match, keep consuming
        continue;
      }

      // consume the rest of the line
      self.skip_until_byte(b'\n');

      if self.peek() == Some('\n') {
        self.advance();
        self.line += 1;
        self.column = 0;
      }
    }
  }

  /// Handle inline backtick code (single or double `\``). Emits a diagnostic
  /// if the matching closing backtick(s) don't appear before EOL.
  fn lex_inline_code(&mut self, count: u8, open_line: usize, open_col: usize) {
    self.emit(TokenKind::CodeStart(count));

    // consume until matching backtick(s) on the same line
    self.skip_until_any2(b'\n', b'`');

    self.emit(TokenKind::Text);

    if self.peek() == Some('`') {
      self.skip_while_byte(b'`');
      self.emit(TokenKind::CodeEnd(self.get_current_lexeme().len() as u8));
    } else {
      self.emit_diagnostic(
        Diagnostic::new(Code::UnterminatedCodeBlock, "unterminated inline code")
          .with_label(Label::primary(
            Span::from_zero_based("", open_line, open_col, count as usize),
            Some("opening backtick here".to_string()),
          ))
          .with_label(Label::secondary(
            Span::from_zero_based("", self.line, self.column, 1),
            Some("expected matching ` before end of line".to_string()),
          ))
          .with_help(r"add a closing ` to the inline code"),
      );
    }
  }
}
