#![no_main]
//! Fuzz target: `Lexer::scan_tokens` must terminate without panic on any
//! UTF-8 input, produce well-formed token spans, and end with `Eof`.

use std::sync::Arc;

use dmc_diagnostic::Code;
use dmc_diagnostic::metadata::{Origin, SourceMeta};
use dmc_lexer::{Lexer, token::TokenKind};
use duck_diagnostic::DiagnosticEngine;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
  let Ok(source) = std::str::from_utf8(data) else { return };

  let meta = Arc::new(SourceMeta { path: Arc::from("<fuzz>"), origin: Origin::Inline("<fuzz>") });
  let mut engine = DiagnosticEngine::<Code>::new();
  let mut lex = Lexer::new(source, meta, &mut engine);
  let _ = lex.scan_tokens();

  // Property 1: stream terminates with Eof.
  let last = lex.tokens.last().expect("at least one token");
  assert!(matches!(last.kind, TokenKind::Eof), "last token not Eof: {:?}", last.kind);

  let src_start = source.as_ptr() as usize;
  let src_end = src_start + source.len();

  for tok in &lex.tokens {
    // Property 2: `raw` borrow lives entirely within `source`.
    let raw_start = tok.raw.as_ptr() as usize;
    let raw_end = raw_start + tok.raw.len();
    assert!(raw_start >= src_start, "token raw before source: {:?}", tok.kind);
    assert!(raw_end <= src_end, "token raw past source end: {:?}", tok.kind);

    // Property 3: span length matches raw length.
    assert_eq!(tok.span.length, tok.raw.len(), "span length != raw length for {:?}", tok.kind);
  }

  // Property 4: bounded growth -- each token besides Eof consumes >=0 bytes
  // and the total cannot exceed `source.len() + 1`. Adversarial inputs can
  // emit many zero-length tokens; cap at 8x to catch runaway loops.
  let upper = source.len().saturating_mul(8) + 64;
  assert!(lex.tokens.len() <= upper, "token explosion: {} tokens for {} bytes", lex.tokens.len(), source.len());
});
