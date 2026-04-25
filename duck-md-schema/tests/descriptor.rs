use duck_md_schema::{compile_descriptor, Ctx};
use serde_json::json;

#[test]
fn compile_string_with_max() {
  let s = compile_descriptor(&json!({"kind": "string", "max": 5})).unwrap();
  let ctx = Ctx::empty();
  assert!(s.parse(&json!("hi"), &ctx).is_ok());
  assert!(s.parse(&json!("toolong"), &ctx).is_err());
}

#[test]
fn compile_object_with_optional_default() {
  let s = compile_descriptor(&json!({
    "kind": "object",
    "fields": {
      "title": { "kind": "string", "max": 10 },
      "draft": { "kind": "default", "inner": { "kind": "boolean" }, "fallback": false },
      "tags":  { "kind": "optional", "inner": { "kind": "array", "item": { "kind": "string" } } }
    }
  })).unwrap();
  let ctx = Ctx::empty();
  let out = s.parse(&json!({"title": "Hi"}), &ctx).unwrap();
  assert_eq!(out["draft"], false);
  assert_eq!(out["title"], "Hi");
  assert!(out.get("tags").is_none());
}

#[test]
fn compile_enum_and_literal() {
  let e = compile_descriptor(&json!({"kind":"enum","variants":["a","b"]})).unwrap();
  let ctx = Ctx::empty();
  assert!(e.parse(&json!("a"), &ctx).is_ok());
  assert!(e.parse(&json!("c"), &ctx).is_err());

  let l = compile_descriptor(&json!({"kind":"literal","expected":42})).unwrap();
  assert!(l.parse(&json!(42), &ctx).is_ok());
  assert!(l.parse(&json!(43), &ctx).is_err());
}

#[test]
fn compile_union_picks_first_match() {
  let u = compile_descriptor(&json!({
    "kind": "union",
    "variants": [
      { "kind": "number" },
      { "kind": "string" }
    ]
  })).unwrap();
  let ctx = Ctx::empty();
  assert!(u.parse(&json!(42), &ctx).is_ok());
  assert!(u.parse(&json!("hi"), &ctx).is_ok());
  assert!(u.parse(&json!(true), &ctx).is_err());
}

#[test]
fn compile_isodate_descriptor() {
  let s = compile_descriptor(&json!({"kind":"isodate"})).unwrap();
  let ctx = Ctx::empty();
  assert!(s.parse(&json!("2024-01-01"), &ctx).is_ok());
  assert!(s.parse(&json!("nope"), &ctx).is_err());
}

#[test]
fn compile_unknown_kind_errors() {
  let r = compile_descriptor(&json!({"kind":"???"}));
  assert!(r.is_err());
}

#[test]
fn compile_path_descriptor_uses_ctx() {
  let s = compile_descriptor(&json!({"kind":"path","removeIndex":true})).unwrap();
  let mut ctx = Ctx::empty();
  ctx.file_path = std::path::PathBuf::from("/root/posts/foo/index.mdx");
  ctx.root = std::path::PathBuf::from("/root");
  let out = s.parse(&serde_json::Value::Null, &ctx).unwrap();
  assert_eq!(out, json!("posts/foo"));
}
