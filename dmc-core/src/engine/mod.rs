use std::path::Path;

use dmc_diagnostic::Code;
use duck_diagnostic::DiagnosticEngine;

use crate::engine::config::EngineConfig;

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
  /// collection in parallel via rayon, then emit the top-level entry
  /// (`index.js` + `index.d.ts`) that re-exports each collection's
  /// `<name>.json`. When `config_path` points at a TS/JS config, the
  /// generated `index.d.ts` infers per-collection record types via
  /// `typeof import(<config>)`.
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
