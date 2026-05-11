#![no_main]
//! Fuzz target: the full `Compiler::compile` pipeline (lex -> parse ->
//! transform passes -> codegen). Must terminate without panic.

use dmc::engine::compile::Compiler;
use dmc_diagnostic::Code;
use duck_diagnostic::DiagnosticEngine;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
  let s = String::from_utf8_lossy(data);
  let mut engine = DiagnosticEngine::<Code>::new();
  let _ = Compiler::compile(&s, &mut engine);
});
