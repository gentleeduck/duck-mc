//! MDX expressions `{ ... }` and MDX comments `{/* ... */}`. Both consume
//! a balanced or comment-terminated body of opaque content; downstream
//! tooling (e.g. acorn) handles the actual JS parsing.

use crate::{Lexer, token::TokenKind};

impl<'eng, 'src: 'eng> Lexer<'eng, 'src> {
  /// MDX expression `{ ... }`. The opening `{` is already consumed.
  /// Tracks string/template/comment state and brace depth so embedded
  /// JS like `{x = "}"}` or `{f({a: 1})}` lexes correctly.
  pub(crate) fn lex_mdx_expression(&mut self) {
    self.emit(TokenKind::ExpressionStart);

    let mut depth: i32 = 1;
    let mut in_string: Option<char> = None;
    let mut in_template = false;
    let mut in_line_comment = false;
    let mut in_block_comment = false;

    while let Some(c) = self.peek() {
      if in_line_comment {
        if c == '\n' {
          in_line_comment = false;
        }
        self.advance();
        continue;
      }
      if in_block_comment {
        if c == '*' && self.peek_next() == Some('/') {
          self.advance();
          self.advance();
          in_block_comment = false;
        } else {
          self.advance();
        }
        continue;
      }
      if let Some(q) = in_string {
        match c {
          '\\' => {
            self.advance();
            self.advance();
          },
          _ if c == q => {
            self.advance();
            in_string = None;
          },
          // Unterminated string at end of line; recover.
          '\n' => {
            self.advance();
            in_string = None;
          },
          _ => {
            self.advance();
          },
        }
        continue;
      }
      if in_template {
        match c {
          '\\' => {
            self.advance();
            self.advance();
          },
          '`' => {
            self.advance();
            in_template = false;
          },
          _ => {
            self.advance();
          },
        }
        continue;
      }

      match c {
        '/' if self.peek_next() == Some('/') => {
          self.advance();
          self.advance();
          in_line_comment = true;
        },
        '/' if self.peek_next() == Some('*') => {
          self.advance();
          self.advance();
          in_block_comment = true;
        },
        '"' | '\'' => {
          in_string = Some(c);
          self.advance();
        },
        '`' => {
          in_template = true;
          self.advance();
        },
        '{' => {
          depth += 1;
          self.advance();
        },
        '}' => {
          depth -= 1;
          if depth == 0 {
            if self.current > self.start {
              self.emit(TokenKind::Text);
            }
            self.advance();
            self.emit(TokenKind::ExpressionEnd);
            return;
          }
          self.advance();
        },
        _ => {
          self.advance();
        },
      }
    }

    // EOF before matching `}` -- emit whatever body we collected.
    if self.current > self.start {
      self.emit(TokenKind::Text);
    }
  }

  /// MDX comment `{/* ... */}`. The opening `{` is already consumed.
  /// Body is verbatim; only `*/}` (with the brace) terminates.
  pub(crate) fn lex_mdx_comment(&mut self) {
    self.advance(); // /
    self.advance(); // *
    self.emit(TokenKind::MdxCommentOpen);

    loop {
      match self.peek() {
        None => {
          if self.current > self.start {
            self.emit(TokenKind::Text);
          }
          return;
        },
        Some('*') if self.peek_next() == Some('/') => {
          // Need `}` immediately after `*/` to terminate.
          let mut iter = self.source[self.current..].chars();
          iter.next();
          iter.next();
          if iter.next() == Some('}') {
            if self.current > self.start {
              self.emit(TokenKind::Text);
            }
            self.advance();
            self.advance();
            self.advance();
            self.emit(TokenKind::MdxCommentClose);
            return;
          }
          self.advance();
        },
        _ => {
          self.advance();
        },
      }
    }
  }
}
