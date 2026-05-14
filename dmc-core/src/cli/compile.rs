use std::path::PathBuf;

use crate::engine::compile::Compiler;
use dmc_diagnostic::{Code, DiagResult};
use duck_diagnostic::{Diagnostic, DiagnosticEngine, diag};

#[derive(clap::Args)]
pub struct CompileCmd {
  #[arg(long, default_value = "dmc.toml")]
  pub path: PathBuf,
}

impl CompileCmd {
  pub fn run(self) -> DiagResult<Diagnostic<Code>> {
    let mut diag_engine = DiagnosticEngine::<Code>::new();

    let src = std::fs::read_to_string(&self.path).map_err(|e| {
      diag!(
        Code::Custom { code: String::from("N001"), severity: duck_diagnostic::Severity::Note },
        format!("read error: {}", e.to_string())
      )
    })?;

    let out = Compiler::compile(&src, &mut diag_engine);
    let json = serde_json::to_string_pretty(&out).unwrap();
    println!("{}", json);

    Ok(diag!(
      Code::Custom { code: String::from("N001"), severity: duck_diagnostic::Severity::Note },
      format!("compiled successfully")
    ))
  }
}
