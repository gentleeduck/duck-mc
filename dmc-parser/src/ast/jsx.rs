use duck_diagnostic::Span;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsxAttr {
  pub name: String,
  pub value: JsxAttrValue,
  pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum JsxAttrValue {
  String(String),
  Expression(String),
  Boolean,
  /// `{...rest}` spread. Stored without the surrounding `{...}`;
  /// `JsxAttr.name` is empty for spreads.
  Spread(String),
}
