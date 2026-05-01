use std::{path::PathBuf, sync::Mutex};

use dmc::compile;
use dmc_diagnostic::Code;
use duck_diagnostic::DiagnosticEngine;

#[derive(clap::Args)]
pub(crate) struct CompileCmd {
  #[arg(long, default_value = "dmc.toml")]
  pub path: PathBuf,
}

impl CompileCmd {
  /// `dmc compile`: read one mdx file, run the default pipeline, print
  /// the resulting `CompileOutput` as pretty JSON to stdout.
  pub(crate) fn run(self, diag_engine: &mut DiagnosticEngine<Code>) -> std::io::Result<()> {
    let src = std::fs::read_to_string(&self.path)?;
    let out = compile(&src, diag_engine);
    let json = serde_json::to_string_pretty(&out).unwrap();
    println!("{}", json);
    Ok(())
  }
}
