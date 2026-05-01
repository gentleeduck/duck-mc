use std::{
  cell::{RefCell, RefMut},
  path::PathBuf,
};

use dmc::Engine;
use dmc_diagnostic::Code;
use duck_diagnostic::{DiagnosticEngine, print_all_smart};

use crate::cli::config::ConfigFile;

/// `dmc build`: load config, run the engine once, print the report.
#[derive(clap::Args)]
pub(crate) struct BuildCmd {
  #[arg(long, default_value = "dmc.toml")]
  pub config: PathBuf,
  #[arg(short, long)]
  pub strict: bool,
  #[arg(long)]
  pub clean: bool,
}

impl BuildCmd {
  /// `dmc build`: load config, run the engine once, print the report.
  /// `strict` aborts on the first validation failure, `clean` wipes
  /// `output_dir` first.
  pub(crate) fn run(self) -> std::io::Result<()> {
    let mut diag_engine = DiagnosticEngine::<Code>::new();

    let mut engine_cfg = ConfigFile::load_engine_cfg(&self.config)?;
    if self.strict {
      engine_cfg.strict = true;
    }
    if self.clean {
      engine_cfg.clean = true;
    }

    Engine::run(&engine_cfg, &mut diag_engine)?;

    // Print every diagnostic at end. Per-diag: with-source when the primary
    // label points at a readable file, compact otherwise (glob/config/IO
    // errors that have no source to snippet).
    print_all_smart(&diag_engine, None);

    Ok(())
  }
}
