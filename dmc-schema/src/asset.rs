use crate::{Ctx, Schema, ValidationError};
use serde_json::{Value, json};
use std::path::PathBuf;

#[derive(Default)]
pub struct FileSchema {
  pub allow_non_relative: bool,
}

impl FileSchema {
  pub fn allow_non_relative(mut self) -> Self {
    self.allow_non_relative = true;
    self
  }
}

impl Schema for FileSchema {
  fn parse(&self, value: &Value, ctx: &Ctx) -> Result<Value, ValidationError> {
    let raw = value.as_str().ok_or_else(|| ValidationError::root("file path must be a string"))?;
    let resolved = resolve_asset(ctx, raw, self.allow_non_relative)?;
    let url = publish_asset(ctx, &resolved)?;
    Ok(Value::String(url))
  }
}

#[derive(Default)]
pub struct ImageSchema {
  pub absolute_root: Option<PathBuf>,
}

impl ImageSchema {
  pub fn absolute_root(mut self, p: impl Into<PathBuf>) -> Self {
    self.absolute_root = Some(p.into());
    self
  }
}

impl Schema for ImageSchema {
  fn parse(&self, value: &Value, ctx: &Ctx) -> Result<Value, ValidationError> {
    let raw = value.as_str().ok_or_else(|| ValidationError::root("image path must be a string"))?;
    let resolved = resolve_asset(ctx, raw, self.absolute_root.is_some())?;
    let url = publish_asset(ctx, &resolved)?;
    let (w, h) = image::image_dimensions(&resolved).unwrap_or((0, 0));
    let mut out = json!({ "src": url, "width": w, "height": h });
    if let Some((dataurl, bw, bh)) = blur_preview(&resolved) {
      let map = out.as_object_mut().unwrap();
      map.insert("blurDataURL".into(), Value::String(dataurl));
      map.insert("blurWidth".into(), Value::from(bw));
      map.insert("blurHeight".into(), Value::from(bh));
    }
    Ok(out)
  }
}

fn blur_preview(path: &PathBuf) -> Option<(String, u32, u32)> {
  use base64::Engine;
  let img = image::open(path).ok()?;
  let target_w: u32 = 8;
  let aspect = img.height() as f32 / img.width() as f32;
  let target_h = (target_w as f32 * aspect).round().max(1.0) as u32;
  let small = img.resize_exact(target_w, target_h, image::imageops::FilterType::Lanczos3);
  let mut buf = Vec::new();
  small.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::WebP).ok()?;
  let b64 = base64::engine::general_purpose::STANDARD.encode(&buf);
  Some((format!("data:image/webp;base64,{b64}"), target_w, target_h))
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
  let joined = dir.join(raw);
  // SEC-004: a relative `../` field can escape the project root. Canonicalize
  // the joined path and assert it stays inside `ctx.root`; reject otherwise.
  // (`canonicalize` resolves `..` and symlinks; the asset must exist anyway
  // since `publish_asset` reads it next.)
  let canonical =
    joined.canonicalize().map_err(|e| ValidationError::root(format!("cannot resolve asset '{raw}': {e}")))?;
  let root_canonical = ctx.root.canonicalize().unwrap_or_else(|_| ctx.root.clone());
  if !canonical.starts_with(&root_canonical) {
    return Err(ValidationError::root(format!(
      "'{raw}' resolves outside the project root and was rejected (path traversal)",
    )));
  }
  Ok(canonical)
}

fn publish_asset(ctx: &Ctx, path: &PathBuf) -> Result<String, ValidationError> {
  let cfg = ctx.assets.as_ref().ok_or_else(|| ValidationError::root("asset pipeline not configured (engine bug?)"))?;
  let bytes =
    std::fs::read(path).map_err(|e| ValidationError::root(format!("cannot read asset {}: {e}", path.display())))?;
  let hash = blake3::hash(&bytes);
  let hash8 = &hash.to_hex().to_string()[..8];
  let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("asset");
  let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("bin");
  let filename = cfg.name_template.replace("[name]", stem).replace("[hash:8]", hash8).replace("[ext]", ext);
  let dest = cfg.assets_dir.join(&filename);
  std::fs::create_dir_all(&cfg.assets_dir)
    .map_err(|e| ValidationError::root(format!("cannot create assets dir: {e}")))?;
  if !dest.exists() {
    std::fs::write(&dest, &bytes)
      .map_err(|e| ValidationError::root(format!("cannot write asset {}: {e}", dest.display())))?;
  }
  let mut url = cfg.base_url.clone();
  if !url.ends_with('/') {
    url.push('/');
  }
  url.push_str(&filename);
  let mut map = cfg.map.lock().unwrap();
  map.insert(path.to_string_lossy().to_string(), url.clone());
  Ok(url)
}

#[cfg(test)]
mod traversal_tests {
  use super::resolve_asset;
  use crate::Ctx;

  /// SEC-004: a relative asset field using `../` must not escape the
  /// project root, even though it is neither a URL nor an absolute path.
  #[test]
  fn rejects_relative_path_traversal() {
    // Build a unique on-disk project root so `canonicalize` succeeds.
    let root = std::env::temp_dir().join(format!("dmc-sec004-{}", std::process::id()));
    let docs = root.join("docs");
    std::fs::create_dir_all(&docs).unwrap();
    std::fs::write(docs.join("page.md"), "x").unwrap();
    // A file that exists outside the root (the traversal target).
    let outside = std::env::temp_dir().join(format!("dmc-sec004-secret-{}", std::process::id()));
    std::fs::write(&outside, "secret").unwrap();

    let ctx = Ctx::new(docs.join("page.md"), root.clone(), String::new());

    // `../<secret>` escapes the project root -> rejected.
    let escaping = format!("../{}", outside.file_name().unwrap().to_string_lossy());
    let err = resolve_asset(&ctx, &escaping, false);
    assert!(err.is_err(), "path traversal `{escaping}` was not rejected");

    // A sibling inside the root still resolves.
    std::fs::write(docs.join("ok.png"), "img").unwrap();
    assert!(resolve_asset(&ctx, "ok.png", false).is_ok());

    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_file(&outside);
  }
}
