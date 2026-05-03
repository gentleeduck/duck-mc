use clap::Parser;
use dmc::cli::{Cli, Cmd, build::BuildCmd, compile::CompileCmd, dev::DevCmd, init::InitCmd};

/// `dmc` CLI entry point.
fn main() -> std::io::Result<()> {
  let cli = Cli::parse();
  match cli.cmd {
    Cmd::Build(args) => BuildCmd::run(args),
    Cmd::Init(args) => InitCmd::run(args),
    Cmd::Compile(args) => CompileCmd::run(args),
    Cmd::Dev(args) => DevCmd::run(args),
  }
}
