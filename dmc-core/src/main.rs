use clap::Parser;
use dmc::cli::{Cli, Cmd, build::BuildCmd, init::InitCmd};
use dmc_diagnostic::{Code, DiagResult};
use duck_diagnostic::DiagnosticEngine;

/// `dmc` CLI entry point.
fn main() -> DiagResult<()> {
  let cli = Cli::parse();
  let mut diag_engine = DiagnosticEngine::<Code>::new();

  let diag = match cli.cmd {
    Cmd::Build(args) => BuildCmd::run(args),
    Cmd::Init(args) => InitCmd::run(args),
    _ => return Ok(()),
    // Cmd::Compile(args) => CompileCmd::run(args),
    // Cmd::Dev(args) => DevCmd::run(args),
  };

  if let Err(e) = diag {
    diag_engine.emit(e);
  }

  // diag_engine.emit(diag);
  diag_engine.print_all_compact();

  Ok(())
}
