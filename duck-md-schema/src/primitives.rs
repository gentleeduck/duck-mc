use serde_json::Value;
use crate::{Ctx, Schema, ValidationError};

#[derive(Default, Clone)]
pub struct StringSchema {
  pub min: Option<usize>,
  pub max: Option<usize>,
  pub regex: Option<String>,
  pub length: Option<usize>,
}

impl StringSchema {
  pub fn min(mut self, n: usize) -> Self { self.min = Some(n); self }
  pub fn max(mut self, n: usize) -> Self { self.max = Some(n); self }
  pub fn length(mut self, n: usize) -> Self { self.length = Some(n); self }
  pub fn regex(mut self, pat: impl Into<String>) -> Self { self.regex = Some(pat.into()); self }
}

impl Schema for StringSchema {
  fn parse(&self, value: &Value, _ctx: &Ctx) -> Result<Value, ValidationError> {
    let s = value.as_str().ok_or_else(|| ValidationError::root(format!(
      "expected string, got {}", json_kind(value),
    )))?;
    let len = s.chars().count();
    if let Some(m) = self.min { if len < m {
      return Err(ValidationError::root(format!("too short (min {m}, got {len})")));
    }}
    if let Some(m) = self.max { if len > m {
      return Err(ValidationError::root(format!("too long (max {m}, got {len})")));
    }}
    if let Some(l) = self.length { if len != l {
      return Err(ValidationError::root(format!("length {l} required (got {len})")));
    }}
    Ok(Value::String(s.to_string()))
  }
}

#[derive(Default, Clone)]
pub struct NumberSchema {
  pub min: Option<f64>,
  pub max: Option<f64>,
  pub int: bool,
}

impl NumberSchema {
  pub fn min(mut self, n: f64) -> Self { self.min = Some(n); self }
  pub fn max(mut self, n: f64) -> Self { self.max = Some(n); self }
  pub fn int(mut self) -> Self { self.int = true; self }
}

impl Schema for NumberSchema {
  fn parse(&self, value: &Value, _ctx: &Ctx) -> Result<Value, ValidationError> {
    let n = value.as_f64().ok_or_else(|| ValidationError::root(format!(
      "expected number, got {}", json_kind(value),
    )))?;
    if self.int && n.fract() != 0.0 {
      return Err(ValidationError::root(format!("expected integer, got {n}")));
    }
    if let Some(m) = self.min { if n < m {
      return Err(ValidationError::root(format!("below min {m} (got {n})")));
    }}
    if let Some(m) = self.max { if n > m {
      return Err(ValidationError::root(format!("above max {m} (got {n})")));
    }}
    Ok(value.clone())
  }
}

#[derive(Default, Clone)]
pub struct BooleanSchema;

impl Schema for BooleanSchema {
  fn parse(&self, value: &Value, _ctx: &Ctx) -> Result<Value, ValidationError> {
    if value.is_boolean() {
      Ok(value.clone())
    } else {
      Err(ValidationError::root(format!(
        "expected boolean, got {}", json_kind(value),
      )))
    }
  }
}

pub struct ArraySchema {
  pub item: Box<dyn Schema>,
  pub min: Option<usize>,
  pub max: Option<usize>,
}

impl ArraySchema {
  pub fn min(mut self, n: usize) -> Self { self.min = Some(n); self }
  pub fn max(mut self, n: usize) -> Self { self.max = Some(n); self }
}

impl Schema for ArraySchema {
  fn parse(&self, value: &Value, ctx: &Ctx) -> Result<Value, ValidationError> {
    let arr = value.as_array().ok_or_else(|| ValidationError::root(format!(
      "expected array, got {}", json_kind(value),
    )))?;
    if let Some(m) = self.min { if arr.len() < m {
      return Err(ValidationError::root(format!("too few items (min {m}, got {})", arr.len())));
    }}
    if let Some(m) = self.max { if arr.len() > m {
      return Err(ValidationError::root(format!("too many items (max {m}, got {})", arr.len())));
    }}
    let mut out = Vec::with_capacity(arr.len());
    for (idx, item) in arr.iter().enumerate() {
      out.push(self.item.parse(item, ctx).map_err(|e| e.at_index(idx))?);
    }
    Ok(Value::Array(out))
  }
}

pub struct ObjectSchema {
  pub fields: Vec<(String, Box<dyn Schema>)>,
  pub passthrough: bool,
}

impl ObjectSchema {
  pub fn passthrough(mut self) -> Self { self.passthrough = true; self }
}

impl Schema for ObjectSchema {
  fn parse(&self, value: &Value, ctx: &Ctx) -> Result<Value, ValidationError> {
    let obj = value.as_object().ok_or_else(|| ValidationError::root(format!(
      "expected object, got {}", json_kind(value),
    )))?;
    let mut out = serde_json::Map::new();
    for (key, schema) in &self.fields {
      let v = obj.get(key).cloned().unwrap_or(Value::Null);
      let parsed = schema.parse(&v, ctx).map_err(|e| e.at(key))?;
      if !parsed.is_null() {
        out.insert(key.clone(), parsed);
      }
    }
    if self.passthrough {
      for (k, v) in obj {
        if !out.contains_key(k) {
          out.insert(k.clone(), v.clone());
        }
      }
    }
    Ok(Value::Object(out))
  }
}

pub struct EnumSchema {
  pub variants: Vec<Value>,
}

impl Schema for EnumSchema {
  fn parse(&self, value: &Value, _ctx: &Ctx) -> Result<Value, ValidationError> {
    if self.variants.contains(value) {
      Ok(value.clone())
    } else {
      let allowed: Vec<String> = self.variants.iter().map(|v| v.to_string()).collect();
      Err(ValidationError::root(format!(
        "must be one of [{}], got {}", allowed.join(", "), value,
      )))
    }
  }
}

pub struct LiteralSchema {
  pub expected: Value,
}

impl Schema for LiteralSchema {
  fn parse(&self, value: &Value, _ctx: &Ctx) -> Result<Value, ValidationError> {
    if value == &self.expected {
      Ok(value.clone())
    } else {
      Err(ValidationError::root(format!(
        "must equal {}, got {}", self.expected, value,
      )))
    }
  }
}

pub struct UnionSchema {
  pub variants: Vec<Box<dyn Schema>>,
}

impl Schema for UnionSchema {
  fn parse(&self, value: &Value, ctx: &Ctx) -> Result<Value, ValidationError> {
    let mut errors = Vec::new();
    for variant in &self.variants {
      match variant.parse(value, ctx) {
        Ok(v) => return Ok(v),
        Err(e) => errors.push(e),
      }
    }
    Err(ValidationError::root(format!(
      "no union variant matched ({} attempts: {})",
      errors.len(),
      errors.iter().map(|e| e.message.clone()).collect::<Vec<_>>().join("; "),
    )))
  }
}

fn json_kind(v: &Value) -> &'static str {
  match v {
    Value::Null => "null",
    Value::Bool(_) => "boolean",
    Value::Number(_) => "number",
    Value::String(_) => "string",
    Value::Array(_) => "array",
    Value::Object(_) => "object",
  }
}
