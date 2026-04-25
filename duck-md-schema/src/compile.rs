use serde_json::Value;
use crate::{markdown::*, modifiers::*, primitives::*, BoxSchema, Schema};

pub fn compile_descriptor(d: &Value) -> Result<Box<dyn Schema>, String> {
  let kind = d.get("kind").and_then(Value::as_str)
    .ok_or_else(|| "schema descriptor missing 'kind'".to_string())?;
  let get_n = |k: &str| d.get(k).and_then(Value::as_u64).map(|n| n as usize);
  let get_f = |k: &str| d.get(k).and_then(Value::as_f64);
  let get_s = |k: &str| d.get(k).and_then(Value::as_str).map(String::from);
  let get_b = |k: &str| d.get(k).and_then(Value::as_bool).unwrap_or(false);

  Ok(match kind {
    "string" => {
      let mut s = StringSchema::default();
      if let Some(n) = get_n("min") { s = s.min(n); }
      if let Some(n) = get_n("max") { s = s.max(n); }
      if let Some(n) = get_n("length") { s = s.length(n); }
      if let Some(p) = get_s("regex") { s = s.regex(p); }
      s.boxed()
    }
    "number" => {
      let mut s = NumberSchema::default();
      if let Some(n) = get_f("min") { s = s.min(n); }
      if let Some(n) = get_f("max") { s = s.max(n); }
      if get_b("int") { s = s.int(); }
      s.boxed()
    }
    "boolean" => BooleanSchema.boxed(),
    "array" => {
      let item = d.get("item").ok_or("array missing 'item'".to_string())?;
      let item_schema = compile_descriptor(item)?;
      let mut a = ArraySchema { item: item_schema, min: None, max: None };
      if let Some(n) = get_n("min") { a = a.min(n); }
      if let Some(n) = get_n("max") { a = a.max(n); }
      a.boxed()
    }
    "object" => {
      let fields_obj = d.get("fields").and_then(Value::as_object)
        .ok_or("object missing 'fields'".to_string())?;
      let mut fields: Vec<(String, Box<dyn Schema>)> = Vec::new();
      for (k, v) in fields_obj {
        fields.push((k.clone(), compile_descriptor(v)?));
      }
      let mut o = ObjectSchema { fields, passthrough: false };
      if get_b("passthrough") { o = o.passthrough(); }
      o.boxed()
    }
    "enum" => {
      let variants = d.get("variants").and_then(Value::as_array).cloned().unwrap_or_default();
      EnumSchema { variants }.boxed()
    }
    "literal" => {
      let expected = d.get("expected").cloned().unwrap_or(Value::Null);
      LiteralSchema { expected }.boxed()
    }
    "union" => {
      let variants = d.get("variants").and_then(Value::as_array)
        .ok_or("union missing 'variants'".to_string())?;
      let inner: Result<Vec<_>, _> = variants.iter().map(compile_descriptor).collect();
      UnionSchema { variants: inner? }.boxed()
    }
    "optional" => {
      let inner = compile_descriptor(d.get("inner").ok_or("optional missing 'inner'".to_string())?)?;
      OptionalSchema { inner }.boxed()
    }
    "nullable" => {
      let inner = compile_descriptor(d.get("inner").ok_or("nullable missing 'inner'".to_string())?)?;
      NullableSchema { inner }.boxed()
    }
    "default" => {
      let inner = compile_descriptor(d.get("inner").ok_or("default missing 'inner'".to_string())?)?;
      let fallback = d.get("fallback").cloned().unwrap_or(Value::Null);
      DefaultSchema { inner, fallback }.boxed()
    }
    "raw" => RawSchema.boxed(),
    "markdown" => MarkdownSchema.boxed(),
    "mdx" => MdxSchema.boxed(),
    "toc" => TocSchema.boxed(),
    "metadata" => MetadataSchema.boxed(),
    "excerpt" => {
      let mut e = ExcerptSchema::default();
      if let Some(n) = get_n("length") { e = e.length(n); }
      e.boxed()
    }
    "path" => {
      let mut p = PathSchema::default();
      if get_b("removeIndex") { p = p.remove_index(); }
      p.boxed()
    }
    "slug" => {
      let mut s = SlugSchema::default();
      if let Some(b) = get_s("bucket") { s = s.by(b); }
      if let Some(r) = d.get("reserved").and_then(Value::as_array) {
        s = s.reserved(r.iter().filter_map(|v| v.as_str().map(String::from)).collect());
      }
      s.boxed()
    }
    "unique" => {
      let mut u = UniqueSchema::default();
      if let Some(b) = get_s("bucket") { u = u.by(b); }
      u.boxed()
    }
    "isodate" => IsodateSchema.boxed(),
    other => return Err(format!("unknown schema kind: {other}")),
  })
}
