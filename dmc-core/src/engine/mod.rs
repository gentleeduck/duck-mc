use dmc_diagnostic::Code;
use duck_diagnostic::DiagnosticEngine;

use crate::engine::{collection::Collection, config::EngineConfig};

pub mod collection;
pub mod config;
pub mod utils;

pub struct Engine;

impl Engine {
  /// Execute one build: optionally clean `output_dir`, process every
  /// collection in parallel via rayon, write per-collection index files,
  /// and return an aggregated [`EngineReport`].
  pub fn run(cfg: &EngineConfig) -> std::io::Result<()> {
    if cfg.clean && cfg.output_dir.exists() {
      std::fs::remove_dir_all(&cfg.output_dir)?;
    }
    std::fs::create_dir_all(&cfg.output_dir)?;

    let mut diag_engine = DiagnosticEngine::<Code>::new();

    // let mut report = EngineReport::default();
    for c in &cfg.collections {
      let r = c.process(cfg, &mut diag_engine)?;
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
