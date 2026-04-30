#![allow(dead_code)]

use dmc_diagnostic::metadata::{Origin, SourceMeta};
use dmc_lexer::{Lexer, token::TokenKind};
use duck_diagnostic::DiagnosticEngine;
use std::cell::RefCell;
use std::sync::Arc;

fn test_meta() -> Arc<SourceMeta> {
  Arc::new(SourceMeta { path: Arc::from("<test>"), version: 0, origin: Origin::Inline("<test>") })
}

pub fn lex_kinds(src: &str) -> Vec<TokenKind> {
  let engine = RefCell::new(DiagnosticEngine::new());
  let mut lexer = Lexer::new(src, test_meta(), engine.borrow_mut());
  let _ = lexer.scan_tokens();
  lexer.tokens.iter().map(|t| t.kind.clone()).collect()
}

pub fn lex_pairs(src: &str) -> Vec<(TokenKind, String)> {
  let engine = RefCell::new(DiagnosticEngine::new());
  let mut lexer = Lexer::new(src, test_meta(), engine.borrow_mut());
  let _ = lexer.scan_tokens();
  lexer.tokens.iter().map(|t| (t.kind.clone(), t.raw.to_string())).collect()
}
