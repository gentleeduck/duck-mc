use std::path::Path;

use dmc_diagnostic::Code;
use duck_diagnostic::DiagnosticEngine;

use crate::engine::config::EngineConfig;

pub mod accumlator;
pub mod collection;
pub mod compile;
pub mod config;
pub mod index;
pub mod schema_ts;
pub mod sidecar;
pub mod utils;

pub struct Engine;

impl Engine {
  /// Execute one build: optionally clean `output_dir`, process every
  /// collection in parallel via rayon, then emit `index.js` + `index.d.ts`
  /// re-exporting each `<name>.json`. With a TS/JS `config_path`, the
  /// generated `index.d.ts` infers record types via `typeof import(...)`.
  pub fn run(
    cfg: &EngineConfig,
    config_path: Option<&Path>,
    diag_engine: &mut DiagnosticEngine<Code>,
  ) -> std::io::Result<()> {
    if cfg.clean && cfg.output_dir.exists() {
      std::fs::remove_dir_all(&cfg.output_dir)?;
    }
    std::fs::create_dir_all(&cfg.output_dir)?;

    for c in &cfg.collections {
      let _ = c.process(cfg, diag_engine);
    }

    let format = cfg.output_format.as_deref().unwrap_or("esm");
    index::write_index(&cfg.output_dir, &cfg.collections, format, config_path)?;

    Ok(())
  }
}
