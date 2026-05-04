use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::engine::{collection::Collection, compile::CompileConfig};

/// Top-level engine config. Drives `Engine::run`: collections, output
/// location, schema strictness, JS plugin hooks (remark/rehype via the
/// Node sidecar), and feature flags such as GFM toggling.
#[derive(Deserialize, Serialize, Default, Clone)]
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

  #[serde(flatten)]
  pub compile: CompileConfig,
}

impl EngineConfig {
  /// Read `dmc.toml` (or a `.ts` / `.js` / `.mjs` config) into an
  /// `EngineConfig`. Routes through `load_ts` for JS-flavoured configs.
  pub(crate) fn load(config_path: &PathBuf) -> std::io::Result<EngineConfig> {
    let ext = config_path.extension().and_then(|s| s.to_str()).unwrap_or("");
    if matches!(ext, "ts" | "js" | "mjs") {
      return Self::load_ts(config_path);
    }
    let raw = std::fs::read_to_string(config_path)?;
    let cfg: EngineConfig =
      toml::from_str(&raw).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;

    Ok(cfg)
  }

  /// Spawn a Node sidecar that imports the user's TS/JS config and prints
  /// the resolved `EngineConfig` as JSON. Lets configs reference JS plugins
  /// (remark / rehype) and runtime helpers.
  fn load_ts(config: &PathBuf) -> std::io::Result<EngineConfig> {
    use std::io::Write;
    let abs = std::fs::canonicalize(config)?;
    let script = include_str!("../../scripts/load-config.mjs");
    let mut tmp = tempfile::Builder::new().suffix(".mjs").tempfile()?;
    tmp.write_all(script.as_bytes())?;
    tmp.flush()?;
    let tmp_path = tmp.path().to_path_buf();

    let attempts: &[(&str, &[&str])] = &[("bun", &[]), ("node", &["--import", "tsx"])];
    let mut last_err: Option<String> = None;
    for (cmd, prefix_args) in attempts {
      let mut c = std::process::Command::new(cmd);
      c.args(*prefix_args).arg(&tmp_path).arg(&abs);
      match c.output() {
        Ok(out) if out.status.success() => {
          let json = String::from_utf8(out.stdout)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
          let cfg: EngineConfig = serde_json::from_str(&json).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, format!("ts config: {e}\n--- output ---\n{json}"))
          })?;
          return Ok(cfg);
        },
        Ok(out) => {
          last_err = Some(format!("{cmd} exit {}: {}", out.status, String::from_utf8_lossy(&out.stderr)));
        },
        Err(e) => last_err = Some(format!("{cmd}: {e}")),
      }
    }
    Err(std::io::Error::new(
      std::io::ErrorKind::NotFound,
      format!("ts config requires `bun` or `node` w/ tsx on PATH ({})", last_err.unwrap_or_default(),),
    ))
  }
}
