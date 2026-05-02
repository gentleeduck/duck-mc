use std::path::PathBuf;

use dmc_diagnostic::Code;
use duck_diagnostic::DiagnosticEngine;

use crate::engine::compile::Compiler;

#[derive(clap::Args)]
pub struct CompileCmd {
  #[arg(long, default_value = "dmc.toml")]
  pub path: PathBuf,
}

impl CompileCmd {
  /// `dmc compile`: read one mdx file, run the default pipeline, print
  /// the resulting `CompileOutput` as pretty JSON to stdout.
  pub fn run(self) -> std::io::Result<()> {
    let mut diag_engine = DiagnosticEngine::<Code>::new();

    let src = std::fs::read_to_string(&self.path)?;
    let out = Compiler::compile(&src, &mut diag_engine);
    let json = serde_json::to_string_pretty(&out).unwrap();
    println!("{}", json);
    Ok(())
  }
}
