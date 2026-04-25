//! Velite-parity schema builder. `s::*` mirrors velite's `s.*` API.
//!
//! ```
//! use duck_md_schema::{s, BoxSchema, Ctx, Schema};
//! use serde_json::json;
//!
//! let schema = s::object(vec![
//!     ("title".into(), s::string().max(99).boxed()),
//!     ("draft".into(), s::default_(s::boolean().boxed(), json!(false)).boxed()),
//! ]);
//! let ctx = Ctx::empty();
//! let out = schema.parse(&json!({"title": "Hello"}), &ctx).unwrap();
//! assert_eq!(out["title"], "Hello");
//! assert_eq!(out["draft"], false);
//! ```

mod asset;
mod compile;
mod ctx;
mod error;
mod markdown;
mod modifiers;
mod primitives;

pub use asset::*;
pub use compile::compile_descriptor;
pub use ctx::{AssetPipeline, Ctx};
pub use error::ValidationError;
pub use markdown::*;
pub use modifiers::*;
pub use primitives::*;

use serde_json::Value;

pub trait Schema: Send + Sync {
  fn parse(&self, value: &Value, ctx: &Ctx) -> Result<Value, ValidationError>;
}

pub mod s {
  use super::*;

  pub fn string() -> StringSchema { StringSchema::default() }
  pub fn number() -> NumberSchema { NumberSchema::default() }
  pub fn boolean() -> BooleanSchema { BooleanSchema }
  pub fn array(item: Box<dyn Schema>) -> ArraySchema {
    ArraySchema { item, min: None, max: None }
  }
  pub fn object(fields: Vec<(String, Box<dyn Schema>)>) -> ObjectSchema {
    ObjectSchema { fields, passthrough: false }
  }
  pub fn enum_(variants: Vec<Value>) -> EnumSchema { EnumSchema { variants } }
  pub fn literal(expected: Value) -> LiteralSchema { LiteralSchema { expected } }
  pub fn union(variants: Vec<Box<dyn Schema>>) -> UnionSchema { UnionSchema { variants } }
  pub fn record(value: Box<dyn Schema>) -> RecordSchema { RecordSchema { value } }
  pub fn tuple(items: Vec<Box<dyn Schema>>) -> TupleSchema { TupleSchema { items } }
  pub fn intersection(left: Box<dyn Schema>, right: Box<dyn Schema>) -> IntersectionSchema {
    IntersectionSchema { left, right }
  }
  pub fn discriminated_union(discriminator: impl Into<String>, variants: Vec<Box<dyn Schema>>) -> DiscriminatedUnionSchema {
    DiscriminatedUnionSchema { discriminator: discriminator.into(), variants }
  }
  pub fn coerce_string() -> CoerceSchema { CoerceSchema { target: CoerceTarget::String } }
  pub fn coerce_number() -> CoerceSchema { CoerceSchema { target: CoerceTarget::Number } }
  pub fn coerce_boolean() -> CoerceSchema { CoerceSchema { target: CoerceTarget::Boolean } }
  pub fn coerce_date() -> CoerceSchema { CoerceSchema { target: CoerceTarget::Date } }

  pub fn optional(inner: Box<dyn Schema>) -> OptionalSchema { OptionalSchema { inner } }
  pub fn nullable(inner: Box<dyn Schema>) -> NullableSchema { NullableSchema { inner } }
  pub fn default_(inner: Box<dyn Schema>, fallback: Value) -> DefaultSchema {
    DefaultSchema { inner, fallback }
  }
  pub fn transform<F>(inner: Box<dyn Schema>, func: F) -> TransformSchema
  where F: Fn(Value) -> Value + Send + Sync + 'static {
    TransformSchema { inner, func: Box::new(func) }
  }
  pub fn refine<F>(inner: Box<dyn Schema>, predicate: F) -> RefineSchema
  where F: Fn(&Value) -> Result<(), String> + Send + Sync + 'static {
    RefineSchema { inner, predicate: Box::new(predicate) }
  }
  pub fn super_refine<F>(inner: Box<dyn Schema>, predicate: F) -> SuperRefineSchema
  where F: Fn(&Value, &mut Vec<String>) + Send + Sync + 'static {
    SuperRefineSchema { inner, predicate: Box::new(predicate) }
  }

  pub fn raw() -> RawSchema { RawSchema }
  pub fn markdown() -> MarkdownSchema { MarkdownSchema }
  pub fn mdx() -> MdxSchema { MdxSchema }
  pub fn toc() -> TocSchema { TocSchema }
  pub fn metadata() -> MetadataSchema { MetadataSchema }
  pub fn excerpt() -> ExcerptSchema { ExcerptSchema::default() }
  pub fn path() -> PathSchema { PathSchema::default() }
  pub fn slug() -> SlugSchema { SlugSchema::default() }
  pub fn unique() -> UniqueSchema { UniqueSchema::default() }
  pub fn isodate() -> IsodateSchema { IsodateSchema }
  pub fn file() -> FileSchema { FileSchema::default() }
  pub fn image() -> ImageSchema { ImageSchema::default() }
}

pub trait BoxSchema: Schema + Sized + 'static {
  fn boxed(self) -> Box<dyn Schema> { Box::new(self) }
}
impl<T: Schema + Sized + 'static> BoxSchema for T {}
