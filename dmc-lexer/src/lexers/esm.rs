//! ESM `import` and `export` statements at column 0. The body is consumed
//! opaquely (with brace/string/comment tracking) up to a top-level newline.

use crate::{Lexer, token::TokenKind};

impl<'eng, 'src: 'eng> Lexer<'eng, 'src> {
  /// Try lexing an `import` or `export` statement at column 0. Returns
  /// `false` if the keyword wasn't actually an MDX ESM declaration.
  pub(crate) fn try_lex_esm(&mut self, keyword: &str) -> bool {
    if self.start_column != 0 {
      return false;
    }
    let rest = &self.source[self.start..];
    if !rest.starts_with(keyword) {
      return false;
    }
    let after = rest.as_bytes().get(keyword.len()).copied();
    if !matches!(after, Some(b' ' | b'\t')) {
      return false;
    }

    // Consume the rest of the keyword. The dispatcher already consumed
    // the first char.
    for _ in 1..keyword.len() {
      self.advance();
    }

    self.consume_esm_body();

    let kind = if keyword == "import" { TokenKind::Import } else { TokenKind::Export };
    self.emit(kind);
    true
  }

  /// Consume an ESM statement body up to a top-level newline. Tracks
  /// strings, template literals, line/block comments, and `{}[]()` depth.
  fn consume_esm_body(&mut self) {
    let mut depth: i32 = 0;
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
          // Unterminated; recover at line end.
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
        '{' | '[' | '(' => {
          depth += 1;
          self.advance();
        },
        '}' | ']' | ')' => {
          depth -= 1;
          self.advance();
        },
        '\n' if depth == 0 => break,
        _ => {
          self.advance();
        },
      }
    }
  }
}
