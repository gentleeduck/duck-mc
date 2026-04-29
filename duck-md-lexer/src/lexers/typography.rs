use duck_diagnostic::{Diagnostic, Label, Span};

use crate::{Lexer, token::TokenKind};
use duck_md_diagnostic::Code;

impl<'eng, 'src: 'eng> Lexer<'eng, 'src> {
  /// Lex an ATX heading marker (`#` x 1-6 followed by space). Falls through
  /// to text if the run isn't followed by a space.
  pub(crate) fn lex_heading(&mut self) {
    let mut level = 1;
    while let Some(c) = self.peek() {
      if c == '#' {
        level += 1;
      } else {
        break;
      }
      self.advance();
    }

    if !self.match_current_char(' ') {
      return self.lex_text();
    }

    self.emit(TokenKind::Heading(level))
  }

  /// Consume a run of plain text up to the next "interesting" character
  /// (delimiter, fence, JSX boundary, escape). Handles backslash escapes for
  /// the standard markdown escapable set.
  pub(crate) fn lex_text(&mut self) {
    while let Some(c) = self.peek() {
      if c == '\\' {
        // look at the next char; if escapable, swallow both and continue
        if let Some(nx) = self.peek_next()
          && matches!(
            nx,
            '\\'
              | '*'
              | '_'
              | '`'
              | '<'
              | '>'
              | '{'
              | '}'
              | '['
              | ']'
              | '('
              | ')'
              | '!'
              | '#'
              | '-'
          )
        {
          self.advance(); // backslash
          self.advance(); // escaped char
          continue;
        }
        // lone backslash, treat as text
        self.advance();
        continue;
      }
      if c == '\n' || c == '`' || c == '{' || c == '[' || c == ']' || c == ')' {
        break;
      }
      if c == '<' {
        if let Some(nx) = self.peek_next()
          && (nx.is_ascii_alphabetic() || nx == '/' || nx == '>')
        {
          break;
        }
        // not JSX, treat `<` as text
        self.advance();
        continue;
      }
      if c == '/' && self.peek_next() == Some('>') {
        break;
      }
      if c == '*' || c == '_' {
        // Unescaped because we already consumed escape pairs above.
        break;
      }
      if c == '~' && self.peek_next() == Some('~') {
        break;
      }
      self.advance();
    }

    self.emit(TokenKind::Text)
  }

  /// Lex a run of `*`. One = italic, two = bold, three = bold or thematic
  /// break depending on whether the line ends right after.
  pub(crate) fn lex_bold(&mut self) {
    // the first '*' is already consumed by caller

    self.skip_while_byte(b'*');
    let at_line_end = self.get_current_char() == Some('\n') || self.is_eof();

    match self.get_current_lexeme() {
      "*" => self.emit(TokenKind::Italic(1)),
      "**" => self.emit(TokenKind::Bold(2)),
      "***" if at_line_end => self.emit(TokenKind::ThematicBreak),
      "***" => self.emit(TokenKind::Bold(3)),
      _ => self.emit(TokenKind::Text),
    }
  }

  /// Lex `~~` as a strikethrough delimiter. Single `~` falls through to text.
  pub(crate) fn lex_strike(&mut self) {
    // first '~' already consumed by caller
    self.skip_while_byte(b'~');
    let lex = self.get_current_lexeme();
    if lex.len() == 2 {
      self.emit(TokenKind::Strike(2));
    } else {
      self.emit(TokenKind::Text);
    }
  }

  /// Lex a run of `_`. One = italic, two = bold, three on EOL = thematic break.
  pub(crate) fn lex_italic(&mut self) {
    // the first '_' is already consumed by caller

    self.skip_while_byte(b'_');
    let c = self.get_current_char();

    match self.get_current_lexeme() {
      "_" => self.emit(TokenKind::Italic(1)),
      "__" => self.emit(TokenKind::Bold(2)),
      "___" if c == Some('\n') => self.emit(TokenKind::ThematicBreak),
      _ => self.emit(TokenKind::Text),
    }
  }

  /// Lex a markdown link `[text](href)`. `[text]` part runs first; the
  /// optional `(href)` is consumed if present. Diagnostic on either side
  /// going unterminated.
  pub(crate) fn lex_link(&mut self) {
    // caller consumed '['; record opener column (one back).
    let open_line = self.line;
    let open_col = self.column.saturating_sub(1);
    self.emit(TokenKind::Bracket);
    self.skip_until_any2(b']', b'\n');
    self.emit(TokenKind::Text);

    if self.get_current_char() != Some(']') {
      self.emit_diagnostic(
        Diagnostic::new(Code::UnterminatedExpression, "unterminated link")
          .with_label(Label::primary(
            Span::from_zero_based("", open_line, open_col, 1),
            Some("link opens here".to_string()),
          ))
          .with_label(Label::secondary(
            Span::from_zero_based("", self.line, self.column, 1),
            Some("expected `]` before end of line".to_string()),
          ))
          .with_help("close the link with `]`"),
      );
      return;
    }

    self.advance();
    self.emit(TokenKind::Bracket);

    // optional `(href)`
    if self.get_current_char() == Some('(') {
      let paren_line = self.line;
      let paren_col = self.column;
      self.advance();
      self.emit(TokenKind::ParenOpen);
      self.skip_until_any2(b')', b'\n');
      self.emit(TokenKind::Text);
      if self.get_current_char() == Some(')') {
        self.advance();
        self.emit(TokenKind::ParenClose);
      } else {
        self.emit_diagnostic(
          Diagnostic::new(Code::UnterminatedExpression, "unterminated link target")
            .with_label(Label::primary(
              Span::from_zero_based("", paren_line, paren_col, 1),
              Some("link target opens here".to_string()),
            ))
            .with_label(Label::secondary(
              Span::from_zero_based("", self.line, self.column, 1),
              Some("expected `)` before end of line".to_string()),
            ))
            .with_help("close the target with `)`"),
        );
      }
    }
  }

  /// Lex `![alt](src)` image syntax. Emits `Bang` then defers to `lex_link`.
  pub(crate) fn lex_image(&mut self) {
    self.emit(TokenKind::Bang);

    if let Some(c) = self.get_current_char()
      && c == '['
    {
      self.advance();
      self.lex_link();
    }
  }

  /// Lex an HTML comment `<!-- ... -->`. Falls back to `lex_text` if `<` is
  /// not actually starting `<!--`.
  pub(crate) fn lex_comment(&mut self) {
    // caller consumed '<', dispatch confirmed peek() == '!'
    // check for <!-- without advancing, so we can fall back cleanly
    if !(self.peek() == Some('!') && self.peek_next() == Some('-')) {
      return self.lex_text();
    }

    // peek further: need the second '-'
    let saved = self.current;
    self.advance(); // !
    self.advance(); // first -
    if self.peek() != Some('-') {
      self.current = saved;
      return self.lex_text();
    }
    self.advance(); // second -
    self.emit(TokenKind::HTMLCommentStart);

    // consume comment content until -->
    loop {
      if self.is_eof() {
        self.emit(TokenKind::Text);
        return;
      }

      if self.peek() == Some('-') && self.peek_next() == Some('-') {
        let content_end = self.current;
        self.advance(); // first -
        self.advance(); // second -
        if self.peek() == Some('>') {
          self.advance(); // >

          // emit content before the -->
          let saved_current = self.current;
          self.current = content_end;
          self.emit(TokenKind::Text);

          // emit -->
          self.start = content_end;
          self.current = saved_current;
          self.emit(TokenKind::HTMLCommentEnd);
          return;
        }
        // not -->, keep going
        continue;
      }

      let c = self.advance();
      if c == '\n' {
        self.line += 1;
        self.column = 0;
      }
    }
  }
}
