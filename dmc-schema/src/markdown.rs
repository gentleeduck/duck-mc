use crate::{Ctx, Schema, ValidationError};
use serde_json::Value;

pub struct RawSchema;

impl Schema for RawSchema {
  fn parse(&self, _value: &Value, ctx: &Ctx) -> Result<Value, ValidationError> {
    Ok(Value::String(ctx.body.clone()))
  }
}

pub struct MarkdownSchema;

impl Schema for MarkdownSchema {
  fn parse(&self, _value: &Value, ctx: &Ctx) -> Result<Value, ValidationError> {
    let html = ctx
      .html
      .clone()
      .ok_or_else(|| ValidationError::root("markdown body not yet rendered (engine bug?)"))?;
    Ok(Value::String(html))
  }
}

pub struct MdxSchema;

impl Schema for MdxSchema {
  fn parse(&self, _value: &Value, ctx: &Ctx) -> Result<Value, ValidationError> {
    let body = ctx
      .mdx_body
      .clone()
      .ok_or_else(|| ValidationError::root("mdx body not yet rendered (engine bug?)"))?;
    Ok(Value::String(body))
  }
}

pub struct TocSchema;

impl Schema for TocSchema {
  fn parse(&self, _value: &Value, ctx: &Ctx) -> Result<Value, ValidationError> {
    Ok(ctx.toc.clone().unwrap_or_else(|| Value::Array(vec![])))
  }
}

pub struct MetadataSchema;

impl Schema for MetadataSchema {
  fn parse(&self, _value: &Value, ctx: &Ctx) -> Result<Value, ValidationError> {
    let plain = ctx.plain_text.clone().unwrap_or_default();
    let words = plain.split_whitespace().count() as u32;
    let reading = ((words as f32) / 200.0).ceil() as u32;
    Ok(serde_json::json!({
      "readingTime": reading.max(1),
      "wordCount": words,
    }))
  }
}

pub struct ExcerptSchema {
  pub length: usize,
}

impl ExcerptSchema {
  pub fn length(mut self, n: usize) -> Self {
    self.length = n;
    self
  }
}

impl Default for ExcerptSchema {
  fn default() -> Self {
    Self { length: 260 }
  }
}

impl Schema for ExcerptSchema {
  fn parse(&self, _value: &Value, ctx: &Ctx) -> Result<Value, ValidationError> {
    let plain = ctx.plain_text.clone().unwrap_or_default();
    let s: String = plain.split_whitespace().collect::<Vec<_>>().join(" ");
    let out = if s.chars().count() <= self.length {
      s
    } else {
      let truncated: String = s.chars().take(self.length).collect();
      format!("{}…", truncated.trim_end())
    };
    Ok(Value::String(out))
  }
}

#[derive(Default)]
pub struct PathSchema {
  pub remove_index: bool,
}

impl PathSchema {
  pub fn remove_index(mut self) -> Self {
    self.remove_index = true;
    self
  }
}

impl Schema for PathSchema {
  fn parse(&self, _value: &Value, ctx: &Ctx) -> Result<Value, ValidationError> {
    let rel = ctx.file_path.strip_prefix(&ctx.root).unwrap_or(&ctx.file_path);
    let mut s = rel.to_string_lossy().to_string();
    s = s.trim_end_matches(".mdx").trim_end_matches(".md").to_string();
    if self.remove_index {
      s = s.trim_end_matches("/index").to_string();
    }
    Ok(Value::String(s))
  }
}

pub struct SlugSchema {
  pub bucket: String,
  pub reserved: Vec<String>,
}

impl SlugSchema {
  pub fn by(mut self, bucket: impl Into<String>) -> Self {
    self.bucket = bucket.into();
    self
  }
  pub fn reserved(mut self, list: Vec<String>) -> Self {
    self.reserved = list;
    self
  }
}

impl Default for SlugSchema {
  fn default() -> Self {
    Self { bucket: "global".into(), reserved: Vec::new() }
  }
}

impl Schema for SlugSchema {
  fn parse(&self, value: &Value, ctx: &Ctx) -> Result<Value, ValidationError> {
    let s = value.as_str().ok_or_else(|| ValidationError::root("slug must be a string"))?;
    if s.len() < 3 || s.len() > 200 {
      return Err(ValidationError::root(format!("slug length must be 3..=200 (got {})", s.len(),)));
    }
    let valid = !s.is_empty()
      && !s.starts_with('-')
      && !s.ends_with('-')
      && !s.contains("--")
      && s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if !valid {
      return Err(ValidationError::root(
        "slug must be kebab-case (lowercase letters, digits, single dashes)",
      ));
    }
    if self.reserved.iter().any(|r| r == s) {
      return Err(ValidationError::root(format!("slug '{s}' is reserved")));
    }
    let key = format!("{}::{s}", self.bucket);
    let mut cache = ctx.unique_cache.lock().unwrap();
    if cache.contains(&key) {
      return Err(ValidationError::root(format!(
        "slug '{s}' already used in bucket '{}'",
        self.bucket
      )));
    }
    cache.insert(key);
    Ok(Value::String(s.to_string()))
  }
}

pub struct UniqueSchema {
  pub bucket: String,
}

impl UniqueSchema {
  pub fn by(mut self, bucket: impl Into<String>) -> Self {
    self.bucket = bucket.into();
    self
  }
}

impl Default for UniqueSchema {
  fn default() -> Self {
    Self { bucket: "global".into() }
  }
}

impl Schema for UniqueSchema {
  fn parse(&self, value: &Value, ctx: &Ctx) -> Result<Value, ValidationError> {
    let s = value.as_str().ok_or_else(|| ValidationError::root("unique value must be a string"))?;
    let key = format!("{}::{s}", self.bucket);
    let mut cache = ctx.unique_cache.lock().unwrap();
    if cache.contains(&key) {
      return Err(ValidationError::root(format!(
        "'{s}' already used in unique bucket '{}'",
        self.bucket
      )));
    }
    cache.insert(key);
    Ok(Value::String(s.to_string()))
  }
}

pub struct IsodateSchema;

impl Schema for IsodateSchema {
  fn parse(&self, value: &Value, _ctx: &Ctx) -> Result<Value, ValidationError> {
    let s = value.as_str().ok_or_else(|| ValidationError::root("isodate must be a string"))?;
    let bytes = s.as_bytes();
    if bytes.len() < 10
      || !bytes[0].is_ascii_digit()
      || !bytes[1].is_ascii_digit()
      || !bytes[2].is_ascii_digit()
      || !bytes[3].is_ascii_digit()
      || bytes[4] != b'-'
      || !bytes[5].is_ascii_digit()
      || !bytes[6].is_ascii_digit()
      || bytes[7] != b'-'
      || !bytes[8].is_ascii_digit()
      || !bytes[9].is_ascii_digit()
    {
      return Err(ValidationError::root(format!("'{s}' is not a valid ISO date")));
    }
    Ok(Value::String(s.to_string()))
  }
}
