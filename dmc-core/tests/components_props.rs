//! Reading JSX attributes into the JSON a schema validates.
//!
//! This is the seam where authored syntax becomes data: `node={1}` has to arrive as the number
//! `1`, not the text `"1"`, or every numeric component schema fails on a correct document.

use dmc::engine::components::props_of;
use dmc_parser::ast::{JsxAttr, JsxAttrValue};
use duck_diagnostic::Span;
use std::sync::Arc;

fn attr(name: &str, value: JsxAttrValue) -> JsxAttr {
  JsxAttr { name: name.to_string(), value, span: Span { file: Arc::from("x.mdx"), line: 7, column: 1, length: 0 } }
}

#[test]
fn reads_expressions_as_json_and_keeps_the_rest_as_text() {
  let props = props_of(&[
    attr("id", JsxAttrValue::String("t1".into())),
    attr("node", JsxAttrValue::Expression("1".into())),
    attr("optional", JsxAttrValue::Expression("true".into())),
    attr("tags", JsxAttrValue::Expression(r#"["a","b"]"#.into())),
    attr("onDone", JsxAttrValue::Expression("handleDone".into())),
    attr("hidden", JsxAttrValue::Boolean),
  ]);
  assert_eq!(props["id"], "t1");
  assert_eq!(props["node"], 1, "a braced number is a number, not the text \"1\"");
  assert_eq!(props["optional"], true);
  assert_eq!(props["tags"], serde_json::json!(["a", "b"]));
  assert_eq!(props["onDone"], "handleDone", "a non-JSON expression is kept, not dropped");
  assert_eq!(props["hidden"], true, "a bare attribute is true, as in JSX");
}

#[test]
fn a_spread_is_skipped_rather_than_named_empty() {
  let props = props_of(&[attr("", JsxAttrValue::Spread("rest".into())), attr("id", JsxAttrValue::String("t".into()))]);
  assert_eq!(props.as_object().unwrap().len(), 1);
  assert_eq!(props["id"], "t");
}

#[test]
fn a_repeated_attribute_takes_the_last_value_as_jsx_does() {
  let props =
    props_of(&[attr("id", JsxAttrValue::String("first".into())), attr("id", JsxAttrValue::String("last".into()))]);
  assert_eq!(props["id"], "last");
}

#[test]
fn whitespace_around_an_expression_does_not_change_its_type() {
  let props = props_of(&[attr("node", JsxAttrValue::Expression("  2  ".into()))]);
  assert_eq!(props["node"], 2);
}

#[test]
fn an_empty_attribute_list_is_an_empty_object_not_null() {
  let props = props_of(&[]);
  assert!(props.is_object(), "a schema expecting an object must not be handed null");
  assert_eq!(props.as_object().unwrap().len(), 0);
}
