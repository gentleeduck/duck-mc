//! Hardening probes for the schema builder API. Validation must accept
//! correct inputs, reject ill-typed ones, and never panic on arbitrary
//! JSON.

use dmc_schema::{BoxSchema, Ctx, Schema, s};
use serde_json::{Value, json};

fn ctx() -> Ctx {
  Ctx::empty()
}

#[test]
fn string_schema_rejects_non_strings() {
  let sc = s::string();
  for v in &[json!(1), json!(null), json!(true), json!([]), json!({})] {
    assert!(sc.parse(v, &ctx()).is_err(), "expected err for {v}");
  }
}

#[test]
fn number_schema_rejects_non_numbers() {
  let sc = s::number();
  for v in &[json!("1"), json!(null), json!(true), json!([]), json!({})] {
    assert!(sc.parse(v, &ctx()).is_err(), "expected err for {v}");
  }
}

#[test]
fn array_schema_rejects_non_arrays() {
  let sc = s::array(s::string().boxed());
  for v in &[json!(1), json!("x"), json!(null), json!({})] {
    assert!(sc.parse(v, &ctx()).is_err(), "expected err for {v}");
  }
}

#[test]
fn object_extra_keys_handled() {
  let sc = s::object(vec![("title".into(), s::string().boxed())]);
  let out = sc.parse(&json!({"title":"hi","extra":1}), &ctx()).unwrap();
  // The schema may or may not strip extras; either behavior is acceptable
  // as long as the declared key survives.
  assert_eq!(out.get("title"), Some(&json!("hi")));
}

#[test]
fn coerce_string_accepts_numbers() {
  let sc = s::coerce_string();
  let out = sc.parse(&json!(42), &ctx()).unwrap();
  assert_eq!(out, json!("42"));
}

#[test]
fn coerce_boolean_accepts_strings() {
  let sc = s::coerce_boolean();
  for (input, expected) in &[("true", true), ("false", false), ("1", true), ("0", false)] {
    let out = sc.parse(&json!(input), &ctx()).unwrap_or_else(|_| panic!("expected coerce to handle {input}"));
    assert_eq!(out, json!(expected), "input={input}");
  }
}

#[test]
fn nested_object_path_reported() {
  let sc = s::object(vec![("outer".into(), s::object(vec![("inner".into(), s::string().min(1).boxed())]).boxed())]);
  let err = sc.parse(&json!({"outer": {"inner": ""}}), &ctx()).unwrap_err();
  assert!(err.path.contains("inner"), "path should reference inner, got {}", err.path);
}

#[test]
fn optional_field_can_be_omitted() {
  let sc = s::object(vec![
    ("title".into(), s::string().boxed()),
    ("subtitle".into(), s::optional(s::string().boxed()).boxed()),
  ]);
  let out = sc.parse(&json!({"title":"x"}), &ctx()).unwrap();
  assert_eq!(out.get("title"), Some(&json!("x")));
}

#[test]
fn default_field_supplies_value_when_missing() {
  let sc = s::object(vec![("draft".into(), s::default_(s::boolean().boxed(), json!(false)).boxed())]);
  let out = sc.parse(&json!({}), &ctx()).unwrap();
  assert_eq!(out.get("draft"), Some(&json!(false)));
}

#[test]
fn schema_does_not_panic_on_arbitrary_json() {
  let sc = s::object(vec![
    ("title".into(), s::string().boxed()),
    ("tags".into(), s::optional(s::array(s::string().boxed()).boxed()).boxed()),
    ("count".into(), s::optional(s::number().boxed()).boxed()),
  ]);
  let weird_inputs: &[Value] = &[
    json!(null),
    json!(0),
    json!("string"),
    json!([]),
    json!({}),
    json!({"title": null}),
    json!({"title": 123}),
    json!({"title": "ok", "tags": "not-an-array"}),
    json!({"title": "ok", "tags": [1, 2, 3]}),
    json!({"title": "ok", "count": "not-a-number"}),
  ];
  for v in weird_inputs {
    let _ = sc.parse(v, &ctx());
  }
}

#[test]
fn enum_only_accepts_listed_values() {
  let sc = s::enum_(vec![json!("a"), json!("b"), json!("c")]);
  assert!(sc.parse(&json!("a"), &ctx()).is_ok());
  assert!(sc.parse(&json!("d"), &ctx()).is_err());
  assert!(sc.parse(&json!(1), &ctx()).is_err());
}

#[test]
fn literal_matches_exact_value_only() {
  let sc = s::literal(json!(42));
  assert!(sc.parse(&json!(42), &ctx()).is_ok());
  assert!(sc.parse(&json!(43), &ctx()).is_err());
  assert!(sc.parse(&json!("42"), &ctx()).is_err());
}
