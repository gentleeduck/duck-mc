#![no_main]
use duck_diagnostic::DiagnosticEngine;
use duck_md_lexer::Lexer;
use libfuzzer_sys::fuzz_target;
use std::cell::RefCell;

fuzz_target!(|data: &[u8]| {
  if let Ok(s) = std::str::from_utf8(data) {
    let engine = RefCell::new(DiagnosticEngine::new());
    let mut lex = Lexer::new(s.to_string(), engine.borrow_mut());
    let _ = lex.scan_tokens();
  }
});
