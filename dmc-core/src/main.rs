use clap::Parser;
use dmc::cli::{Cli, Cmd, build::BuildCmd, init::InitCmd};
use dmc_diagnostic::{Code, DiagResult};
use duck_diagnostic::DiagnosticEngine;

#[allow(clippy::result_large_err)]
fn main() -> DiagResult<()> {
  let cli = Cli::parse();
  let mut diag_engine = DiagnosticEngine::<Code>::new();

  let diag = match cli.cmd {
    Cmd::Build(args) => BuildCmd::run(args),
    Cmd::Init(args) => InitCmd::run(args),
    _ => return Ok(()),
  };

  if let Err(e) = diag {
    diag_engine.emit(e);
  }

  diag_engine.print_all_compact();

  Ok(())
}
