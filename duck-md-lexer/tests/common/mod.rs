#![allow(dead_code)]

use std::cell::RefCell;
use duck_diagnostic::DiagnosticEngine;
use duck_md_lexer::{Lexer, token::TokenKind};

pub fn lex_kinds(src: &str) -> Vec<TokenKind> {
    let engine = RefCell::new(DiagnosticEngine::new());
    let mut lexer = Lexer::new(src.to_string(), engine.borrow_mut());
    let _ = lexer.scan_tokens();
    lexer.tokens.iter().map(|t| t.kind.clone()).collect()
}

pub fn lex_pairs(src: &str) -> Vec<(TokenKind, String)> {
    let engine = RefCell::new(DiagnosticEngine::new());
    let mut lexer = Lexer::new(src.to_string(), engine.borrow_mut());
    let _ = lexer.scan_tokens();
    lexer.tokens.iter().map(|t| (t.kind.clone(), t.raw.clone())).collect()
}
