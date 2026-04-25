use duck_diagnostic::Span;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsxAttr {
  pub name: String,
  pub value: JsxAttrValue,
  #[serde(skip, default = "crate::default_span")]
  pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum JsxAttrValue {
  String(String),
  Expression(String),
  Boolean,
}
