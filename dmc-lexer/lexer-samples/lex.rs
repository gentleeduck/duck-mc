//! Lex one MDX/MD file and dump tokens + diagnostics with source context.
//!
//! Usage:
//!   cargo run -p dmc-lexer --bin lexer -- <path>
//!   cargo run -p dmc-lexer --bin lexer -- ./tmp/docs/index.mdx
//!
//! Output:
//!   1. tokens (one per line: line:col, kind, raw lexeme)
//!   2. diagnostics rendered via `print_all_smart` — each one shows the
//!      source line + caret under the offending span.

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use dmc_diagnostic::Code;
use dmc_diagnostic::metadata::{Origin, SourceMeta};
use dmc_lexer::Lexer;
use duck_diagnostic::DiagnosticEngine;

fn main() -> ExitCode {
  let path = match std::env::args().nth(1).map(PathBuf::from) {
    Some(p) => p,
    None => {
      eprintln!("usage: lexer <path>");
      return ExitCode::from(2);
    },
  };

  let source = match std::fs::read_to_string(&path) {
    Ok(s) => s,
    Err(e) => {
      eprintln!("read {}: {}", path.display(), e);
      return ExitCode::from(2);
    },
  };

  let meta = Arc::new(SourceMeta { path: Arc::from(path.display().to_string()), origin: Origin::File(path.clone()) });

  let mut engine = DiagnosticEngine::<Code>::new();
  let mut lexer = Lexer::new(&source, meta, &mut engine);
  let _ = lexer.scan_tokens();

  println!("=== tokens ({}) ===", lexer.tokens.len());
  for t in &lexer.tokens {
    println!("{:>4}:{:<3} {:<24} {:?}", t.span.line, t.span.column, format!("{:?}", t.kind), t.raw,);
  }

  if engine.iter().len() > 0 {
    println!(
      "\n=== diagnostics ({}, {} errors / {} warnings) ===",
      engine.iter().len(),
      engine.error_count(),
      engine.warning_count(),
    );
    engine.print_all(&source);
  } else {
    println!("\n=== diagnostics: none ===");
  }

  if engine.error_count() + engine.bug_count() > 0 { ExitCode::from(1) } else { ExitCode::SUCCESS }
}
