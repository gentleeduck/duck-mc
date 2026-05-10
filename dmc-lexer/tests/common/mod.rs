#![allow(dead_code)]

use dmc_diagnostic::metadata::{Origin, SourceMeta};
use dmc_lexer::{Lexer, token::TokenKind};
use duck_diagnostic::DiagnosticEngine;
use std::sync::Arc;

fn test_meta() -> Arc<SourceMeta> {
  Arc::new(SourceMeta { path: Arc::from("<test>"), origin: Origin::Inline("<test>") })
}

/// Lex `src` and return only the token kinds (no spans/raw).
pub fn lex_kinds(src: &str) -> Vec<TokenKind> {
  let mut engine = DiagnosticEngine::new();
  let mut lexer = Lexer::new(src, test_meta(), &mut engine);
  let _ = lexer.scan_tokens();
  lexer.tokens.iter().map(|t| t.kind.clone()).collect()
}

/// Lex `src` and return `(kind, raw)` pairs.
pub fn lex_pairs(src: &str) -> Vec<(TokenKind, String)> {
  let mut engine = DiagnosticEngine::new();
  let mut lexer = Lexer::new(src, test_meta(), &mut engine);
  let _ = lexer.scan_tokens();
  lexer.tokens.iter().map(|t| (t.kind.clone(), t.raw.to_string())).collect()
}

/// Drop trivia (whitespace, breaks, blank lines, EOF) so test assertions
/// can focus on structural tokens.
pub fn lex_significant(src: &str) -> Vec<TokenKind> {
  lex_kinds(src).into_iter().filter(|k| !k.is_trivia() && !matches!(k, TokenKind::Eof)).collect()
}
