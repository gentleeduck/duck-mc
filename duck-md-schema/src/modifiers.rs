use serde_json::Value;
use crate::{Ctx, Schema, ValidationError};

pub struct OptionalSchema {
  pub inner: Box<dyn Schema>,
}

impl Schema for OptionalSchema {
  fn parse(&self, value: &Value, ctx: &Ctx) -> Result<Value, ValidationError> {
    if value.is_null() {
      Ok(Value::Null)
    } else {
      self.inner.parse(value, ctx)
    }
  }
}

pub struct NullableSchema {
  pub inner: Box<dyn Schema>,
}

impl Schema for NullableSchema {
  fn parse(&self, value: &Value, ctx: &Ctx) -> Result<Value, ValidationError> {
    if value.is_null() {
      Ok(Value::Null)
    } else {
      self.inner.parse(value, ctx)
    }
  }
}

pub struct DefaultSchema {
  pub inner: Box<dyn Schema>,
  pub fallback: Value,
}

impl Schema for DefaultSchema {
  fn parse(&self, value: &Value, ctx: &Ctx) -> Result<Value, ValidationError> {
    if value.is_null() {
      self.inner.parse(&self.fallback, ctx)
    } else {
      self.inner.parse(value, ctx)
    }
  }
}

pub struct TransformSchema {
  pub inner: Box<dyn Schema>,
  pub func: Box<dyn Fn(Value) -> Value + Send + Sync>,
}

impl Schema for TransformSchema {
  fn parse(&self, value: &Value, ctx: &Ctx) -> Result<Value, ValidationError> {
    let parsed = self.inner.parse(value, ctx)?;
    Ok((self.func)(parsed))
  }
}

pub struct RefineSchema {
  pub inner: Box<dyn Schema>,
  pub predicate: Box<dyn Fn(&Value) -> Result<(), String> + Send + Sync>,
}

impl Schema for RefineSchema {
  fn parse(&self, value: &Value, ctx: &Ctx) -> Result<Value, ValidationError> {
    let parsed = self.inner.parse(value, ctx)?;
    (self.predicate)(&parsed).map_err(ValidationError::root)?;
    Ok(parsed)
  }
}
