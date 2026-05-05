use std::path::Path;

use dmc_diagnostic::Code;
use duck_diagnostic::DiagnosticEngine;

use crate::engine::config::EngineConfig;

pub mod accumulator;
pub mod cache;
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

    // Warm the math (KaTeX/MathML) cache from disk so previously-rendered
    // expressions skip the JS engine entirely on this build.
    let math_cache_path = cfg.output_dir.join(".cache").join("math.json");
    #[cfg(feature = "math")]
    if cfg.cache_enabled {
      dmc_transform::Math::load_cache(&math_cache_path);
    }

    for c in &cfg.collections {
      let _ = c.process(cfg, diag_engine);
    }

    // Flush math cache so the next build starts warm.
    #[cfg(feature = "math")]
    if cfg.cache_enabled {
      dmc_transform::Math::save_cache(&math_cache_path);
    }
    let _ = math_cache_path;

    let format = cfg.output_format.as_deref().unwrap_or("esm");
    index::write_index(&cfg.output_dir, &cfg.collections, format, config_path)?;

    Ok(())
  }
}
