use clap::Parser;

use crate::cli::{Cli, Cmd, build::BuildCmd, compile::CompileCmd, dev::DevCmd, init::InitCmd};
mod cli;

/// `dmc` CLI entry point. Dispatches the parsed subcommand.
fn main() -> std::io::Result<()> {
  let cli = Cli::parse();
  match cli.cmd {
    Cmd::Build(args) => BuildCmd::run(args),
    _ => todo!(),
    // Cmd::Init(args) => InitCmd::run(args),
    // Cmd::Compile(args) => CompileCmd::run(args),
    // Cmd::Dev(args) => DevCmd::run(args),
  }
}
