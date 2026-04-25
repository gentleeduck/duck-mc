use serde_json::{json, Value};
use std::path::PathBuf;
use crate::{Ctx, Schema, ValidationError};

pub struct FileSchema {
  pub allow_non_relative: bool,
}

impl FileSchema {
  pub fn allow_non_relative(mut self) -> Self { self.allow_non_relative = true; self }
}

impl Default for FileSchema {
  fn default() -> Self { Self { allow_non_relative: false } }
}

impl Schema for FileSchema {
  fn parse(&self, value: &Value, ctx: &Ctx) -> Result<Value, ValidationError> {
    let raw = value.as_str().ok_or_else(||
      ValidationError::root("file path must be a string")
    )?;
    let resolved = resolve_asset(ctx, raw, self.allow_non_relative)?;
    let url = ctx_publish_asset(ctx, &resolved)?;
    Ok(Value::String(url))
  }
}

pub struct ImageSchema {
  pub absolute_root: Option<PathBuf>,
}

impl ImageSchema {
  pub fn absolute_root(mut self, p: impl Into<PathBuf>) -> Self {
    self.absolute_root = Some(p.into()); self
  }
}

impl Default for ImageSchema {
  fn default() -> Self { Self { absolute_root: None } }
}

impl Schema for ImageSchema {
  fn parse(&self, value: &Value, ctx: &Ctx) -> Result<Value, ValidationError> {
    let raw = value.as_str().ok_or_else(||
      ValidationError::root("image path must be a string")
    )?;
    let resolved = resolve_asset(ctx, raw, self.absolute_root.is_some())?;
    let url = ctx_publish_asset(ctx, &resolved)?;
    let (w, h) = match image::image_dimensions(&resolved) {
      Ok(d) => (d.0, d.1),
      Err(_) => (0, 0),
    };
    Ok(json!({ "src": url, "width": w, "height": h }))
  }
}

fn resolve_asset(ctx: &Ctx, raw: &str, allow_abs: bool) -> Result<PathBuf, ValidationError> {
  if raw.starts_with("http://") || raw.starts_with("https://") || raw.starts_with("//") {
    return Err(ValidationError::root(format!("'{raw}' is a URL, not a local file")));
  }
  if raw.starts_with('/') {
    if !allow_abs {
      return Err(ValidationError::root(format!(
        "'{raw}' is not relative; pass allowNonRelativePath / absoluteRoot to permit",
      )));
    }
    return Ok(PathBuf::from(raw));
  }
  let dir = ctx.file_path.parent().unwrap_or(&ctx.root);
  Ok(dir.join(raw))
}

fn ctx_publish_asset(ctx: &Ctx, path: &PathBuf) -> Result<String, ValidationError> {
  let cfg = ctx.assets.as_ref().ok_or_else(||
    ValidationError::root("asset pipeline not configured (engine bug?)")
  )?;
  let bytes = std::fs::read(path).map_err(|e|
    ValidationError::root(format!("cannot read asset {}: {e}", path.display()))
  )?;
  let hash = blake3::hash(&bytes);
  let hash8 = &hash.to_hex().to_string()[..8];
  let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("asset");
  let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("bin");
  let filename = cfg.name_template
    .replace("[name]", stem)
    .replace("[hash:8]", hash8)
    .replace("[ext]", ext);
  let dest = cfg.assets_dir.join(&filename);
  std::fs::create_dir_all(&cfg.assets_dir).map_err(|e|
    ValidationError::root(format!("cannot create assets dir: {e}"))
  )?;
  if !dest.exists() {
    std::fs::write(&dest, &bytes).map_err(|e|
      ValidationError::root(format!("cannot write asset {}: {e}", dest.display()))
    )?;
  }
  let mut url = cfg.base_url.clone();
  if !url.ends_with('/') { url.push('/'); }
  url.push_str(&filename);
  let mut map = cfg.map.lock().unwrap();
  map.insert(path.to_string_lossy().to_string(), url.clone());
  Ok(url)
}
