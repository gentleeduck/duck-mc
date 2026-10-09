//! Build-time extraction of embedded components.
//!
//! A collection's `components` map names the JSX a body may embed. Those nodes are emitted onto
//! the record so a consumer reads them as data, without parsing MDX or running a browser. This is
//! another `NodeSink`, so it rides the existing walk rather than adding a pass.

use dmc_codegen::{NodeSink, WalkCtx};
use dmc_parser::ast::{JsxAttr, JsxAttrValue, Node};
use serde_json::{Map, Value};
use std::collections::HashSet;

/// One component found in a body, before its props are validated.
#[derive(Debug, Clone)]
pub struct Found {
  pub name: String,
  pub props: Value,
  /// 1-based, from the node's own span, so a diagnostic points at the element.
  pub line: usize,
  pub column: usize,
  /// Index in this list of the nearest enclosing collected component, if any.
  pub parent: Option<usize>,
}

/// Collects the JSX nodes whose names are configured for a collection.
pub struct ComponentCollector<'a> {
  names: &'a HashSet<String>,
  pub found: Vec<Found>,
  /// Indices of collected elements currently open, innermost last.
  open: Vec<usize>,
}

impl<'a> ComponentCollector<'a> {
  pub fn new(names: &'a HashSet<String>) -> Self {
    Self { names, found: Vec::new(), open: Vec::new() }
  }

  fn push(&mut self, name: &str, attrs: &[JsxAttr], line: usize, column: usize) -> usize {
    let parent = self.open.last().copied();
    self.found.push(Found { name: name.to_string(), props: props_of(attrs), line, column, parent });
    self.found.len() - 1
  }
}

impl NodeSink for ComponentCollector<'_> {
  fn enter(&mut self, node: &Node, _ctx: &WalkCtx) {
    match node {
      Node::JsxElement(e) if self.names.contains(&e.name) => {
        let i = self.push(&e.name, &e.attrs, e.span.line, e.span.column);
        // children are walked next, so anything collected inside points back at this one
        self.open.push(i);
      },
      Node::JsxSelfClosing(e) if self.names.contains(&e.name) => {
        self.push(&e.name, &e.attrs, e.span.line, e.span.column);
      },
      _ => {},
    }
  }

  fn leave(&mut self, node: &Node, _ctx: &WalkCtx) {
    if let Node::JsxElement(e) = node
      && self.names.contains(&e.name)
    {
      self.open.pop();
    }
  }
}

/// Attributes as the JSON object `dmc_schema` validates.
///
/// A braced expression is read as JSON first, so `{3}` is the number 3 and not the text "3".
/// Anything that is not JSON — an identifier, a call — is kept as a string rather than dropped.
pub fn props_of(attrs: &[JsxAttr]) -> Value {
  let mut map = Map::new();
  for a in attrs {
    // `{...rest}` carries no name and no statically known value
    if a.name.is_empty() {
      continue;
    }
    let v = match &a.value {
      JsxAttrValue::String(s) => Value::String(s.clone()),
      JsxAttrValue::Boolean => Value::Bool(true),
      JsxAttrValue::Expression(e) => {
        let t = e.trim();
        serde_json::from_str(t).unwrap_or_else(|_| Value::String(t.to_string()))
      },
      JsxAttrValue::Spread(_) => continue,
    };
    map.insert(a.name.clone(), v);
  }
  Value::Object(map)
}
