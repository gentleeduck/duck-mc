use std::path::Path;

use dmc_diagnostic::{Code, DiagResult};
use duck_diagnostic::{DiagnosticEngine, diag};
use rayon::prelude::*;

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
  pub fn run(cfg: &EngineConfig, config_path: Option<&Path>, diag_engine: &mut DiagnosticEngine<Code>) -> DiagResult {
    if cfg.clean && cfg.output_dir.exists() {
      let paths: Vec<_> = ["index.js", "index.d.ts", "index.cjs"]
        .iter()
        .map(|n| cfg.output_dir.join(n))
        .chain(cfg.collections.iter().map(|c| cfg.output_dir.join(format!("{}.json", c.name))))
        .collect();

      let errors: Vec<_> = paths
        .par_iter()
        .filter_map(|p| match std::fs::remove_file(p) {
          Err(e) if e.kind() != std::io::ErrorKind::NotFound => Some(e),
          _ => None,
        })
        .collect();

      for e in errors {
        diag_engine.emit(diag!(Code::IoWrite, format!("clean: remove failed: {e}")));
      }
    }

    std::fs::create_dir_all(&cfg.output_dir).map_err(|e| {
      diag!(
        Code::Custom { code: String::from("N001"), severity: duck_diagnostic::Severity::Note },
        format!("output_dir error: {}", e.to_string())
      )
    })?;

    let math_cache_path = cfg.output_dir.join(".cache").join("math.json");
    #[cfg(feature = "math")]
    if cfg.cache_enabled {
      dmc_transform::Math::load_cache(&math_cache_path)?;
    }

    for c in &cfg.collections {
      let _ = c.process(cfg, diag_engine);
    }

    // Flush math cache so the next build starts warm.
    #[cfg(feature = "math")]
    if cfg.cache_enabled {
      dmc_transform::Math::save_cache(&math_cache_path)?;
    }
    let _ = math_cache_path;

    let format = cfg.output_format.as_deref().unwrap_or("esm");
    index::write_index(&cfg.output_dir, &cfg.collections, format, config_path)?;

    Ok(())
  }
}
