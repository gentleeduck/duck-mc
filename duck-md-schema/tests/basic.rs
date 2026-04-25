use duck_md_schema::{s, BoxSchema, Ctx, Schema};
use pretty_assertions::assert_eq;
use serde_json::json;

fn ctx() -> Ctx { Ctx::empty() }

#[test]
fn string_min_max() {
  let schema = s::string().min(2).max(5);
  assert_eq!(schema.parse(&json!("hi"), &ctx()).unwrap(), json!("hi"));
  assert!(schema.parse(&json!("a"), &ctx()).is_err());
  assert!(schema.parse(&json!("toolong"), &ctx()).is_err());
  assert!(schema.parse(&json!(42), &ctx()).is_err());
}

#[test]
fn number_int_range() {
  let schema = s::number().int().min(1.0).max(10.0);
  assert_eq!(schema.parse(&json!(5), &ctx()).unwrap(), json!(5));
  assert!(schema.parse(&json!(0), &ctx()).is_err());
  assert!(schema.parse(&json!(11), &ctx()).is_err());
  assert!(schema.parse(&json!(2.5), &ctx()).is_err());
}

#[test]
fn boolean_strict() {
  let schema = s::boolean();
  assert_eq!(schema.parse(&json!(true), &ctx()).unwrap(), json!(true));
  assert!(schema.parse(&json!(1), &ctx()).is_err());
  assert!(schema.parse(&json!("true"), &ctx()).is_err());
}

#[test]
fn array_of_strings() {
  let schema = s::array(s::string().min(1).boxed()).min(1);
  assert_eq!(
    schema.parse(&json!(["a", "b"]), &ctx()).unwrap(),
    json!(["a", "b"]),
  );
  let err = schema.parse(&json!([""]), &ctx()).unwrap_err();
  assert_eq!(err.path, "[0]");
}

#[test]
fn object_with_optional_and_default() {
  let schema = s::object(vec![
    ("title".into(), s::string().max(99).boxed()),
    ("draft".into(), s::default_(s::boolean().boxed(), json!(false)).boxed()),
    ("tags".into(), s::optional(s::array(s::string().boxed()).boxed()).boxed()),
  ]);
  let out = schema.parse(&json!({"title": "Hello"}), &ctx()).unwrap();
  assert_eq!(out["title"], "Hello");
  assert_eq!(out["draft"], false);
  assert!(out.get("tags").is_none()); // optional + omitted → not present

  let out = schema
    .parse(&json!({"title": "Hi", "tags": ["a"]}), &ctx())
    .unwrap();
  assert_eq!(out["tags"], json!(["a"]));
}

#[test]
fn nested_object_path_in_error() {
  let schema = s::object(vec![
    (
      "author".into(),
      s::object(vec![("name".into(), s::string().min(1).boxed())]).boxed(),
    ),
  ]);
  let err = schema
    .parse(&json!({"author": {"name": ""}}), &ctx())
    .unwrap_err();
  assert_eq!(err.path, "author.name");
}

#[test]
fn enum_and_literal() {
  let schema = s::enum_(vec![json!("draft"), json!("published")]);
  assert_eq!(schema.parse(&json!("draft"), &ctx()).unwrap(), json!("draft"));
  assert!(schema.parse(&json!("other"), &ctx()).is_err());

  let schema = s::literal(json!(42));
  assert_eq!(schema.parse(&json!(42), &ctx()).unwrap(), json!(42));
  assert!(schema.parse(&json!(43), &ctx()).is_err());
}

#[test]
fn refine_and_transform() {
  let schema = s::transform(
    s::refine(s::string().boxed(), |v| {
      if v.as_str().unwrap().contains(' ') {
        Err("must not contain space".into())
      } else {
        Ok(())
      }
    }).boxed(),
    |v| serde_json::Value::String(v.as_str().unwrap().to_uppercase()),
  );
  assert_eq!(schema.parse(&json!("hello"), &ctx()).unwrap(), json!("HELLO"));
  assert!(schema.parse(&json!("a b"), &ctx()).is_err());
}

#[test]
fn isodate_validates() {
  let schema = s::isodate();
  assert_eq!(
    schema.parse(&json!("2024-01-01"), &ctx()).unwrap(),
    json!("2024-01-01"),
  );
  assert_eq!(
    schema.parse(&json!("2024-01-01T12:34:56Z"), &ctx()).unwrap(),
    json!("2024-01-01T12:34:56Z"),
  );
  assert!(schema.parse(&json!("not-a-date"), &ctx()).is_err());
}

#[test]
fn slug_kebab_check() {
  let schema = s::slug();
  assert_eq!(schema.parse(&json!("my-post"), &ctx()).unwrap(), json!("my-post"));
  let mut ctx2 = ctx();
  schema.parse(&json!("my-post"), &ctx2).unwrap();
  // duplicate in same context
  assert!(schema.parse(&json!("my-post"), &mut ctx2).is_err());
  // bad shape
  assert!(schema.parse(&json!("MyPost"), &ctx()).is_err());
  assert!(schema.parse(&json!("my--post"), &ctx()).is_err());
  assert!(schema.parse(&json!("ab"), &ctx()).is_err());
}

#[test]
fn unique_dedupes() {
  let schema = s::unique().by("posts");
  let ctx2 = ctx();
  schema.parse(&json!("hello"), &ctx2).unwrap();
  assert!(schema.parse(&json!("hello"), &ctx2).is_err());
  // different bucket — independent
  let other = s::unique().by("authors");
  other.parse(&json!("hello"), &ctx2).unwrap();
}

#[test]
fn metadata_and_excerpt_use_ctx() {
  use duck_md_schema::Ctx;
  let mut c = Ctx::empty();
  c.plain_text = Some("alpha beta gamma delta epsilon zeta".repeat(20));

  let m = s::metadata().parse(&json!(null), &c).unwrap();
  assert!(m["wordCount"].as_u64().unwrap() >= 100);
  assert!(m["readingTime"].as_u64().unwrap() >= 1);

  let e = s::excerpt().length(20).parse(&json!(null), &c).unwrap();
  assert!(e.as_str().unwrap().ends_with('…'));
}
