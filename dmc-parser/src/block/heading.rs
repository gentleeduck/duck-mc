use crate::ast::*;
use crate::parser::Parser;
use dmc_lexer::token::TokenKind;

impl<'eng, 'tokens> Parser<'eng, 'tokens> {
  /// ATX heading. Anchor slug derived lazily via `Heading::slug()`.
  pub(super) fn parse_heading(&mut self) -> Node {
    let span = self.current_span();
    let level = match self.peek_kind() {
      Some(TokenKind::Heading(n)) => *n,
      _ => 1,
    };
    self.advance();
    let mut children = self.collect_inline_until_break();
    // CM 4.2: trailing-spaces hard breaks don't render as `<br />` inside
    // a heading - drop them along with any whitespace-only tail text.
    while matches!(children.last(), Some(Node::HardBreak(_))) {
      children.pop();
    }
    // Strip the leading post-`#` space the lexer leaves in the inline stream.
    if let Some(Node::Text(t)) = children.first_mut() {
      let trimmed = t.value.trim_start_matches([' ', '\t']).to_string();
      if trimmed.is_empty() {
        children.remove(0);
      } else {
        t.value = trimmed;
      }
    }
    // Strip trailing whitespace-only text left by the `HeadingTrailingHashes`
    // skip + the spaces between text and the optional `###`.
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

  /// `Some(1)` for `=`, `Some(2)` for `-`, else `None`. No cursor change.
  pub(super) fn setext_underline_level(&self) -> Option<u8> {
    let t = self.tokens.get(self.pos)?;
    match &t.kind {
      TokenKind::SetextUnderline(_) => Some(1),
      TokenKind::ThematicBreak => {
        // CM 4.3: setext H2 = all-`-` with optional trailing whitespace.
        let trimmed = t.raw.trim_end_matches([' ', '\t']);
        if !trimmed.is_empty() && trimmed.chars().all(|c| c == '-') { Some(2) } else { None }
      },
      _ => None,
    }
  }

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
