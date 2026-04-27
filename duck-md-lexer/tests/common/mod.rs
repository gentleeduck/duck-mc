#![allow(dead_code)]

use duck_diagnostic::DiagnosticEngine;
use duck_md_lexer::{Lexer, token::TokenKind};
use std::cell::RefCell;

pub fn lex_kinds(src: &str) -> Vec<TokenKind> {
  let engine = RefCell::new(DiagnosticEngine::new());
  let mut lexer = Lexer::new(src, engine.borrow_mut());
  let _ = lexer.scan_tokens();
  lexer.tokens.iter().map(|t| t.kind.clone()).collect()
}

pub fn lex_pairs(src: &str) -> Vec<(TokenKind, String)> {
  let engine = RefCell::new(DiagnosticEngine::new());
  let mut lexer = Lexer::new(src, engine.borrow_mut());
  let _ = lexer.scan_tokens();
  lexer.tokens.iter().map(|t| (t.kind.clone(), t.raw.to_string())).collect()
}
