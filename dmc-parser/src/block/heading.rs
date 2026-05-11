use crate::ast::*;
use crate::parser::Parser;
use dmc_lexer::token::TokenKind;

impl<'eng, 'tokens> Parser<'eng, 'tokens> {
  /// ATX heading. Anchor slug is derived lazily via `Heading::slug()`.
  pub(super) fn parse_heading(&mut self) -> Node {
    let span = self.current_span();
    let level = match self.peek_kind() {
      Some(TokenKind::Heading(n)) => *n,
      _ => 1,
    };
    self.advance();
    let mut children = self.collect_inline_until_break();
    // CM 4.2: drop trailing HardBreak / whitespace-only text from a
    // heading. Trailing-spaces hard-break detection by the lexer fires
    // on the `   \n` at the end of `# foo   `, which the spec doesn't
    // turn into a `<br />` inside a heading.
    while matches!(children.last(), Some(Node::HardBreak(_))) {
      children.pop();
    }
    // The lexer leaves the post-`#` space in the inline stream as a Text /
    // Whitespace node, so the first heading-text node ends up with a leading
    // space ("` Inline marks`"). Strip it to match velite / rehype output.
    if let Some(Node::Text(t)) = children.first_mut() {
      let trimmed = t.value.trim_start_matches([' ', '\t']).to_string();
      if trimmed.is_empty() {
        children.remove(0);
      } else {
        t.value = trimmed;
      }
    }
    // Strip trailing whitespace-only text nodes (left by the HeadingTrailingHashes
    // skip + spaces between the text and the optional `###`).
    while let Some(Node::Text(t)) = children.last_mut() {
      let trimmed = t.value.trim_end_matches([' ', '\t']).to_string();
      if trimmed.is_empty() {
        children.pop();
      } else if trimmed.len() != t.value.len() {
        t.value = trimmed;
        break;
      } else {
        break;
      }
    }
    Node::Heading(Heading { level, children, span, id: None })
  }

  /// `Some(1)` for an `=` underline, `Some(2)` for a `-` underline, else
  /// `None`. Cursor is left untouched.
  pub(super) fn setext_underline_level(&self) -> Option<u8> {
    let t = self.tokens.get(self.pos)?;
    match &t.kind {
      TokenKind::SetextUnderline(_) => Some(1),
      TokenKind::ThematicBreak => {
        // CM 4.3: setext H2 = run of `-` plus optional trailing
        // whitespace. Trim trailing ws then verify all-dashes.
        let trimmed = t.raw.trim_end_matches([' ', '\t']);
        if !trimmed.is_empty() && trimmed.chars().all(|c| c == '-') { Some(2) } else { None }
      },
      _ => None,
    }
  }

  /// Consume the underline tokens that `setext_underline_level` matched.
  pub(super) fn eat_setext_underline(&mut self) {
    if let Some(t) = self.tokens.get(self.pos) {
      match t.kind {
        TokenKind::SetextUnderline(_) | TokenKind::ThematicBreak => {
          self.advance();
        },
        _ => {},
      }
    }
  }
}
