//! Lex one .mdx file (or stdin) and print every diagnostic the lexer raises.
//!
//!     cargo run -p dmc-lexer --bin lexer -- ../samples/index.mdx
//!     cargo run -p dmc-lexer --bin lexer                       # loops over ../samples/errors/
//!     echo '`oops' | cargo run -p dmc-lexer --bin lexer        # stdin
//!
//! Flags:
//!     --tokens   dump the token stream as a plain table
//!     --json     dump tokens as a structured JSON document (kind, span, raw)
//!     --quiet    suppress diagnostics (only show tokens / json)

use dmc_diagnostic::metadata::{Origin, SourceMeta};
use dmc_lexer::Lexer;
use dmc_lexer::token::Token;
use duck_diagnostic::DiagnosticEngine;
use serde_json::{Value, json};
use std::io::{self, Read};
use std::path::PathBuf;
use std::sync::Arc;

fn main() -> io::Result<()> {
  let mut args: Vec<String> = std::env::args().skip(1).collect();
  let show_tokens = take_flag(&mut args, "--tokens");
  let show_json = take_flag(&mut args, "--json");
  let quiet = take_flag(&mut args, "--quiet");

  let mode = Mode { show_tokens, show_json, quiet };

  if args.is_empty() && atty_stdin() {
    // no arg + no piped stdin -> loop over every error sample in the shared
    // samples/errors dir at the workspace root.
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("samples").join("errors");
    let mut entries: Vec<PathBuf> = std::fs::read_dir(&dir)?
      .filter_map(|e| e.ok())
      .map(|e| e.path())
      .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("mdx"))
      .collect();
    entries.sort();
    let mut totals = (0usize, 0usize);
    for path in &entries {
      let (errs, warns) = run_one(path, &mode)?;
      totals.0 += errs;
      totals.1 += warns;
    }
    if !mode.show_json {
      println!("summary: {} sample(s), {} total error(s), {} total warning(s)", entries.len(), totals.0, totals.1);
    }
    return Ok(());
  }

  let source = if let Some(path) = args.first() {
    return run_one(&PathBuf::from(path), &mode).map(|_| ());
  } else {
    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf)?;
    buf
  };
  let stdin_meta = Arc::new(SourceMeta { path: Arc::from("<stdin>"), version: 0, origin: Origin::Stdin });
  lex_and_print("<stdin>", &source, &mode, stdin_meta);
  Ok(())
}

struct Mode {
  show_tokens: bool,
  show_json: bool,
  quiet: bool,
}

fn run_one(path: &PathBuf, mode: &Mode) -> io::Result<(usize, usize)> {
  let source = std::fs::read_to_string(path)?;
  let label = path.file_name().unwrap().to_string_lossy().into_owned();
  let meta = Arc::new(SourceMeta { path: Arc::from(label.clone()), version: 0, origin: Origin::File(path.clone()) });
  Ok(lex_and_print(&label, &source, mode, meta))
}

fn lex_and_print(label: &str, source: &str, mode: &Mode, meta: Arc<SourceMeta>) -> (usize, usize) {
  let mut engine = DiagnosticEngine::new();
  let mut lexer = Lexer::new(source, meta, &mut engine);
  let _ = lexer.scan_tokens();
  let tokens = std::mem::take(&mut lexer.tokens);
  drop(lexer);
  let (e, w) = (engine.error_count(), engine.warning_count());

  if mode.show_json {
    let doc = build_json(label, &tokens, e, w);
    println!("{}", serde_json::to_string_pretty(&doc).unwrap_or_else(|_| "{}".into()));
    return (e, w);
  }

  println!("=== {} ===", label);
  if mode.show_tokens {
    print_token_table(&tokens);
  }
  if !mode.quiet {
    if engine.is_empty() {
      println!("(no diagnostics)\n");
      return (0, 0);
    }
    engine.print_all(source);
    println!("-> {} error(s), {} warning(s)\n", e, w);
  }
  (e, w)
}

fn build_json(label: &str, tokens: &[Token], errors: usize, warnings: usize) -> Value {
  let toks: Vec<Value> = tokens
    .iter()
    .enumerate()
    .map(|(i, t)| {
      json!({
        "index": i,
        "kind": format!("{:?}", t.kind),
        "raw": t.raw,
        "span": {
          "file": t.span.file,
          "line": t.span.line,
          "column": t.span.column,
          "length": t.span.length,
        },
      })
    })
    .collect();
  json!({
    "label": label,
    "tokenCount": tokens.len(),
    "errors": errors,
    "warnings": warnings,
    "tokens": toks,
  })
}

fn print_token_table(tokens: &[Token]) {
  if tokens.is_empty() {
    return;
  }
  let kinds: Vec<String> = tokens.iter().map(|t| format!("{:?}", t.kind)).collect();
  let positions: Vec<String> = tokens.iter().map(|t| format!("{}:{}", t.span.line, t.span.column)).collect();
  let lens: Vec<String> = tokens.iter().map(|t| t.span.length.to_string()).collect();
  let raws: Vec<String> = tokens.iter().map(|t| format!("{:?}", t.raw)).collect();

  let kw = kinds.iter().map(|s| s.len()).max().unwrap_or(0).max(4);
  let pw = positions.iter().map(|s| s.len()).max().unwrap_or(0).max(3);
  let lw = lens.iter().map(|s| s.len()).max().unwrap_or(0).max(3);

  println!("  {:<kw$}  {:>pw$}  {:>lw$}  RAW", "KIND", "POS", "LEN", kw = kw, pw = pw, lw = lw);
  println!("  {:-<kw$}  {:->pw$}  {:->lw$}  {:-<8}", "", "", "", "", kw = kw, pw = pw, lw = lw);
  for ((kind, pos), (len, raw)) in kinds.iter().zip(positions.iter()).zip(lens.iter().zip(raws.iter())) {
    println!("  {:<kw$}  {:>pw$}  {:>lw$}  {}", kind, pos, len, raw, kw = kw, pw = pw, lw = lw);
  }
  println!("  {} tokens", tokens.len());
}

fn take_flag(args: &mut Vec<String>, name: &str) -> bool {
  let had = args.iter().any(|a| a == name);
  args.retain(|a| a != name);
  had
}

fn atty_stdin() -> bool {
  // Best-effort: if stdin is a terminal, treat as "no piped input".
  // We use the cheap libc-free check via /proc.
  std::fs::metadata("/proc/self/fd/0").map(|m| !m.file_type().is_fifo() && !m.file_type().is_socket()).unwrap_or(true)
}

#[cfg(target_family = "unix")]
use std::os::unix::fs::FileTypeExt;
