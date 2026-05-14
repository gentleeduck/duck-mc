use dmc_diagnostic::{Code, DiagResult};
use duck_diagnostic::diag;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::engine::{collection::Collection, compile::CompileConfig};

/// Top-level engine config consumed by `Engine::run`.
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct EngineConfig {
  pub root: PathBuf,
  pub output_dir: PathBuf,
  pub output_name: Option<String>,
  pub output_format: Option<String>,
  pub clean: bool,
  pub strict: bool,
  pub collections: Vec<Collection>,
  pub include_html: bool,
  /// Persist per-file compile output to `<output_dir>/.cache/dmc/`.
  pub cache_enabled: bool,

  #[serde(flatten)]
  pub compile: CompileConfig,
}

impl Default for EngineConfig {
  fn default() -> Self {
    Self {
      root: PathBuf::new(),
      output_dir: PathBuf::new(),
      output_name: None,
      output_format: None,
      clean: false,
      strict: false,
      collections: Vec::new(),
      include_html: false,
      cache_enabled: true,
      compile: CompileConfig::default(),
    }
  }
}

impl EngineConfig {
  pub(crate) fn load(config_path: &PathBuf) -> DiagResult<EngineConfig> {
    let raw = std::fs::read_to_string(config_path)
      .map_err(|e| diag!(Code::InvalidConfigPath, format!("config error: {}", e.to_string())))?;

    let cfg: EngineConfig =
      toml::from_str(&raw).map_err(|e| diag!(Code::InvalidConfig, format!("config error: {}", e.to_string())))?;

    Ok(cfg)
  }
}
