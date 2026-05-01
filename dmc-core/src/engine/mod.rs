use dmc_diagnostic::Code;
use duck_diagnostic::{DiagnosticEngine, print_all_smart};

use crate::engine::config::EngineConfig;

pub mod collection;
pub mod config;
pub mod sidecar;
pub mod utils;

pub struct Engine;

impl Engine {
  /// Execute one build: optionally clean `output_dir`, process every
  /// collection in parallel via rayon, write per-collection index files,
  /// and return an aggregated [`EngineReport`].
  pub fn run(cfg: &EngineConfig, diag_engine: &mut DiagnosticEngine<Code>) -> std::io::Result<()> {
    if cfg.clean && cfg.output_dir.exists() {
      std::fs::remove_dir_all(&cfg.output_dir)?;
    }
    std::fs::create_dir_all(&cfg.output_dir)?;

    // let mut report = EngineReport::default();
    for c in &cfg.collections {
      let _r = c.process(cfg, diag_engine);
    }

    // write_index(&cfg.output_dir, &report, cfg.output_format.as_deref().unwrap_or("esm"))?;

    // if cfg.strict && !report.errors.is_empty() {
    //   let first = &report.errors[0];
    //   return Err(std::io::Error::new(
    //     std::io::ErrorKind::InvalidData,
    //     format!("validation failed in strict mode: {}: {}", first.file.display(), first.message),
    //   ));
    // }
    Ok(())
  }
}
