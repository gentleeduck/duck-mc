use crate::{Lexer, token::TokenKind};

impl<'eng, 'src: 'eng> Lexer<'eng, 'src> {
  /// Lex a numeric run that introduces an ordered list item (`12.`, `1)`).
  pub(crate) fn lex_ordered_list_item(&mut self) {
    self.skip_while_ascii(|b| b.is_ascii_digit());
    self.emit(TokenKind::OrderedListItem);
  }

  /// Lex a `-` or run of `-` that introduces an unordered list item.
  pub(crate) fn lex_unordered_list_item(&mut self) {
    self.skip_while_byte(b'-');
    self.emit(TokenKind::UnorderedListItem);
  }
}
