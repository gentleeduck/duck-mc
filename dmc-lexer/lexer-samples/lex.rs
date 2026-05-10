//! Lex one MDX/MD file and dump tokens + diagnostics with source context.
//!
//! Usage:
//!   cargo run -p dmc-lexer --bin lexer -- <path>
//!   cargo run -p dmc-lexer --bin lexer -- <path> --bench [iters]
//!
//! Output:
//!   1. tokens (one per line: line:col, kind, raw lexeme)
//!   2. diagnostics rendered via `print_all` -- each one shows the source
//!      line + caret under the offending span.
//!
//! With `--bench`, prints throughput stats instead.

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Instant;

use dmc_diagnostic::Code;
use dmc_diagnostic::metadata::{Origin, SourceMeta};
use dmc_lexer::Lexer;
use duck_diagnostic::DiagnosticEngine;

fn main() -> ExitCode {
  let mut args = std::env::args().skip(1);
  let path = match args.next().map(PathBuf::from) {
    Some(p) => p,
    None => {
      eprintln!("usage: lexer <path> [--bench [iters]]");
      return ExitCode::from(2);
    },
  };

  let bench_iters: Option<u32> = match args.next().as_deref() {
    Some("--bench") => Some(args.next().and_then(|s| s.parse().ok()).unwrap_or(10_000)),
    Some(other) => {
      eprintln!("unknown arg: {other}");
      return ExitCode::from(2);
    },
    None => None,
  };

  let source = match std::fs::read_to_string(&path) {
    Ok(s) => s,
    Err(e) => {
      eprintln!("read {}: {}", path.display(), e);
      return ExitCode::from(2);
    },
  };

  let meta = Arc::new(SourceMeta { path: Arc::from(path.display().to_string()), origin: Origin::File(path.clone()) });

  if let Some(iters) = bench_iters {
    return run_bench(&source, &meta, iters);
  }

  let mut engine = DiagnosticEngine::<Code>::new();
  let mut lexer = Lexer::new(&source, meta, &mut engine);
  let _ = lexer.scan_tokens();

  println!("=== tokens ({}) ===", lexer.tokens.len());
  for t in &lexer.tokens {
    println!("{:>4}:{:<3} {:<24} {:?}", t.span.line, t.span.column, format!("{:?}", t.kind), t.raw);
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

fn run_bench(source: &str, meta: &Arc<SourceMeta>, iters: u32) -> ExitCode {
  // Warmup.
  let warmup = (iters / 20).max(50);
  for _ in 0..warmup {
    let mut engine = DiagnosticEngine::<Code>::new();
    let mut lexer = Lexer::new(source, meta.clone(), &mut engine);
    let _ = lexer.scan_tokens();
    std::hint::black_box(&lexer.tokens);
  }

  // Token count snapshot for reporting.
  let mut engine = DiagnosticEngine::<Code>::new();
  let mut lexer = Lexer::new(source, meta.clone(), &mut engine);
  let _ = lexer.scan_tokens();
  let token_count = lexer.tokens.len();

  let bytes = source.len();
  let start = Instant::now();
  for _ in 0..iters {
    let mut engine = DiagnosticEngine::<Code>::new();
    let mut lexer = Lexer::new(source, meta.clone(), &mut engine);
    let _ = lexer.scan_tokens();
    std::hint::black_box(&lexer.tokens);
  }
  let elapsed = start.elapsed();

  let total_ns = elapsed.as_nanos() as f64;
  let per_run_ns = total_ns / f64::from(iters);
  let mb_per_s = (bytes as f64 * f64::from(iters)) / total_ns / 1.048_576e-3;
  let ns_per_byte = per_run_ns / bytes.max(1) as f64;
  let ns_per_token = per_run_ns / token_count.max(1) as f64;

  println!("bench: {iters} iters over {bytes} bytes -> {token_count} tokens");
  println!("       total       = {:.2} ms", total_ns / 1e6);
  println!("       per run     = {per_run_ns:.0} ns ({:.2} us)", per_run_ns / 1000.0);
  println!("       throughput  = {mb_per_s:.1} MiB/s");
  println!("       ns / byte   = {ns_per_byte:.2}");
  println!("       ns / token  = {ns_per_token:.2}");
  ExitCode::SUCCESS
}
