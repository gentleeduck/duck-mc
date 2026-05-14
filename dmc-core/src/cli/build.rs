use std::path::PathBuf;

use crate::{Engine, engine::config::EngineConfig};
use dmc_diagnostic::{Code, DiagResult};
use duck_diagnostic::{Diagnostic, DiagnosticEngine, diag};

/// `dmc build`: load config, run the engine once, print the report.
#[derive(clap::Args)]
pub struct BuildCmd {
  #[arg(long, default_value = "dmc.toml")]
  pub config: PathBuf,
  #[arg(short, long)]
  pub strict: bool,
  #[arg(long)]
  pub clean: bool,
}

impl BuildCmd {
  pub fn run(self) -> DiagResult<Diagnostic<Code>> {
    let mut diag_engine = DiagnosticEngine::<Code>::new();
    let started = std::time::Instant::now();

    let mut engine_cfg = EngineConfig::load(&self.config)?;

    if self.strict {
      engine_cfg.strict = true;
    }
    if self.clean {
      engine_cfg.clean = true;
    }

    Engine::run(&engine_cfg, Some(&self.config), &mut diag_engine)?;

    Ok(diag!(
      Code::Custom { code: String::from("N001"), severity: duck_diagnostic::Severity::Note },
      format!("built successfully in {:<.3?}", started.elapsed())
    ))
  }
}
