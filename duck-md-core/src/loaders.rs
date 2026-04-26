use serde_json::Value;
use std::path::Path;

pub struct Loaded {
  pub data: Value,
  pub content: String,
}

pub trait Loader: Send + Sync {
  fn test(&self, path: &Path) -> bool;
  fn load(&self, path: &Path, source: &str) -> Result<Loaded, String>;
}

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

pub struct LoaderRegistry {
  loaders: Vec<Box<dyn Loader>>,
}

impl LoaderRegistry {
  pub fn with_defaults() -> Self {
    Self { loaders: vec![Box::new(MatterLoader), Box::new(YamlLoader), Box::new(JsonLoader)] }
  }

  pub fn pick(&self, path: &Path) -> Option<&dyn Loader> {
    self.loaders.iter().find(|l| l.test(path)).map(|l| l.as_ref())
  }
}
