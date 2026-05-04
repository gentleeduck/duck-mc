//! Per-extension loaders: bytes -> `Loaded { data, content }` for schema
//! validation. `MatterLoader` runs the full mdx compile; `YamlLoader` /
//! `JsonLoader` parse data files directly.

use dmc_diagnostic::Code;
use duck_diagnostic::DiagnosticEngine;
use serde_json::Value;
use std::path::Path;

use crate::engine::compile::Compiler;

/// One loaded source file: schema-validated `data` (frontmatter for mdx,
/// the whole doc for yaml/json) plus the original `content` string.
pub struct Loaded {
  pub data: Value,
  pub content: String,
}

/// Pluggable per-extension loader: `test` claims a path, `load` parses it.
pub trait Loader: Send + Sync {
  fn test(&self, path: &Path) -> bool;
  fn load(&self, path: &Path, source: &str, diag_engine: &mut DiagnosticEngine<Code>) -> Result<Loaded, String>;
}

/// `.md` / `.mdx` / `.markdown` loader. Runs the full compile and stashes
/// the `CompileOutput` under `data.__compiled` so the schema can refine it
/// (e.g. `transform: ctx => ctx.html`).
pub struct MatterLoader;

impl Loader for MatterLoader {
  fn test(&self, path: &Path) -> bool {
    matches!(path.extension().and_then(|s| s.to_str()), Some("md") | Some("mdx") | Some("markdown"))
  }

  fn load(&self, _path: &Path, source: &str, diag_engine: &mut DiagnosticEngine<Code>) -> Result<Loaded, String> {
    let out = Compiler::compile(source, diag_engine);
    let mut data = if let Value::Object(_) = out.frontmatter {
      out.frontmatter.clone()
    } else {
      Value::Object(serde_json::Map::new())
    };
    if let Value::Object(map) = &mut data {
      map.insert("__compiled".into(), serde_json::to_value(&out).unwrap_or(Value::Null));
    }
    Ok(Loaded { data, content: source.to_string() })
  }
}

/// `.yaml` / `.yml` loader. Parses to `serde_yaml::Value`, then converts
/// to `serde_json::Value` for schema interop.
pub struct YamlLoader;

impl Loader for YamlLoader {
  fn test(&self, path: &Path) -> bool {
    matches!(path.extension().and_then(|s| s.to_str()), Some("yaml") | Some("yml"))
  }

  fn load(&self, _path: &Path, source: &str, _diag_engine: &mut DiagnosticEngine<Code>) -> Result<Loaded, String> {
    let v: serde_yaml::Value = serde_yaml::from_str(source).map_err(|e| format!("yaml parse: {e}"))?;
    let json = serde_json::to_value(v).map_err(|e| format!("yaml→json: {e}"))?;
    Ok(Loaded { data: json, content: source.to_string() })
  }
}

/// `.json` loader. Straight `serde_json::from_str`.
pub struct JsonLoader;

impl Loader for JsonLoader {
  fn test(&self, path: &Path) -> bool {
    matches!(path.extension().and_then(|s| s.to_str()), Some("json"))
  }

  fn load(&self, _path: &Path, source: &str, _diag_engine: &mut DiagnosticEngine<Code>) -> Result<Loaded, String> {
    let v: Value = serde_json::from_str(source).map_err(|e| format!("json parse: {e}"))?;
    Ok(Loaded { data: v, content: source.to_string() })
  }
}

/// Ordered loader chain; first match wins. Defaults: Matter, Yaml, Json.
pub struct LoaderRegistry {
  loaders: Vec<Box<dyn Loader>>,
}

impl LoaderRegistry {
  /// Registry pre-loaded with the three built-in loaders.
  pub fn with_defaults() -> Self {
    Self { loaders: vec![Box::new(MatterLoader), Box::new(YamlLoader), Box::new(JsonLoader)] }
  }

  /// First loader whose `test()` accepts `path`, or `None`.
  pub fn pick(&self, path: &Path) -> Option<&dyn Loader> {
    self.loaders.iter().find(|l| l.test(path)).map(|l| l.as_ref())
  }
}
