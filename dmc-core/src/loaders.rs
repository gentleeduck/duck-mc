//! Per-extension source loaders that turn a file's raw bytes into a
//! `Loaded { data, content }` pair the engine can hand to schema validation.
//! `MatterLoader` runs the full mdx compile, `YamlLoader` / `JsonLoader`
//! parse data files directly.

use serde_json::Value;
use std::path::Path;

/// Result of loading one source file: the structured data the schema
/// validates against (frontmatter for mdx, the whole doc for yaml/json),
/// plus the original `content` string for downstream consumers.
pub struct Loaded {
  pub data: Value,
  pub content: String,
}

/// Pluggable file-type loader. `test` decides whether this loader handles
/// the given path; `load` does the parse.
pub trait Loader: Send + Sync {
  fn test(&self, path: &Path) -> bool;
  fn load(&self, path: &Path, source: &str) -> Result<Loaded, String>;
}

/// Loader for `.md` / `.mdx` / `.markdown` — runs the full compile and
/// stashes the entire `CompileOutput` under `data.__compiled` so the schema
/// can refine it (e.g. `transform: ctx => ctx.html`).
pub struct MatterLoader;

impl Loader for MatterLoader {
  fn test(&self, path: &Path) -> bool {
    matches!(path.extension().and_then(|s| s.to_str()), Some("md") | Some("mdx") | Some("markdown"))
  }

  fn load(&self, _path: &Path, source: &str) -> Result<Loaded, String> {
    let out = crate::compile(source);
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

/// Loader for `.yaml` / `.yml` — parses to `serde_yaml::Value`, then
/// converts to `serde_json::Value` for schema interop.
pub struct YamlLoader;

impl Loader for YamlLoader {
  fn test(&self, path: &Path) -> bool {
    matches!(path.extension().and_then(|s| s.to_str()), Some("yaml") | Some("yml"))
  }

  fn load(&self, _path: &Path, source: &str) -> Result<Loaded, String> {
    let v: serde_yaml::Value =
      serde_yaml::from_str(source).map_err(|e| format!("yaml parse: {e}"))?;
    let json = serde_json::to_value(v).map_err(|e| format!("yaml→json: {e}"))?;
    Ok(Loaded { data: json, content: source.to_string() })
  }
}

/// Loader for `.json` — straight `serde_json::from_str`.
pub struct JsonLoader;

impl Loader for JsonLoader {
  fn test(&self, path: &Path) -> bool {
    matches!(path.extension().and_then(|s| s.to_str()), Some("json"))
  }

  fn load(&self, _path: &Path, source: &str) -> Result<Loaded, String> {
    let v: Value = serde_json::from_str(source).map_err(|e| format!("json parse: {e}"))?;
    Ok(Loaded { data: v, content: source.to_string() })
  }
}

/// Ordered list of loaders consulted in turn — first match wins. Default
/// registers Matter / Yaml / Json in that order.
pub struct LoaderRegistry {
  loaders: Vec<Box<dyn Loader>>,
}

impl LoaderRegistry {
  /// Registry pre-loaded with the three built-in loaders.
  pub fn with_defaults() -> Self {
    Self { loaders: vec![Box::new(MatterLoader), Box::new(YamlLoader), Box::new(JsonLoader)] }
  }

  /// Pick the first loader whose `test()` accepts `path`, or `None`.
  pub fn pick(&self, path: &Path) -> Option<&dyn Loader> {
    self.loaders.iter().find(|l| l.test(path)).map(|l| l.as_ref())
  }
}
