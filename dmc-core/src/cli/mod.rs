use clap::{Parser, Subcommand};

use crate::cli::{build::BuildCmd, compile::CompileCmd, dev::DevCmd, init::InitCmd};

pub mod build;
pub mod compile;
pub mod config;
pub mod dev;
pub mod init;

#[derive(Parser)]
#[command(name = "dmc `dmc`", version, about = "Rust MDX compiler")]
pub(crate) struct Cli {
  #[command(subcommand)]
  pub cmd: Cmd,
}

#[derive(Subcommand)]
pub(crate) enum Cmd {
  /// Build all collections from dmc.toml.
  Build(BuildCmd),
  /// Scaffold a default dmc.toml in the current directory.
  Init(InitCmd),
  /// Compile a single MDX file and print JSON to stdout.
  Compile(CompileCmd),
  /// Watch the content roots and rebuild on change.
  Dev(DevCmd),
}
