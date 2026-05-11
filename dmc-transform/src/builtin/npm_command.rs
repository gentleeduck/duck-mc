//! `npm install` / `npx` -> `<PackageManagerTabs>`. See
//! `transformers/npm-command.md` for full docs.

use crate::pipeline::Transformer;
use crate::visit::{NodeAction, Visitor, walk_root};
use dmc_diagnostic::Code;
use dmc_diagnostic::metadata::SourceMeta;
use dmc_parser::ast::*;

/// Detect `npm install ...`, `npx create-...`, and `npx ...` first-lines in
/// fenced code blocks and replace them with a `<PackageManagerTabs>` JSX
/// element carrying the per-pm equivalents as plain string attrs.
#[derive(Default)]
pub struct NpmCommand;

impl NpmCommand {
  pub fn new() -> Self {
    Self
  }
}

impl Transformer for NpmCommand {
  fn name(&self) -> &str {
    "npm-command"
  }

  fn transform(
    &self,
    doc: &mut Document,
    _meta: &SourceMeta,
    _diag_engine: &mut duck_diagnostic::DiagnosticEngine<Code>,
  ) {
    let mut v = Apply;
    walk_root(&mut doc.children, &mut v);
  }
}

struct Apply;

impl NpmCommand {
  /// Map the block's first line to npm/yarn/pnpm/bun equivalents. Mirrors
  /// velite's `rehype-npm-command`: any `npm install`, `npx create-`, or
  /// `npx <bin>` first-line triggers the swap.
  fn derive(value: &str) -> Option<[(&'static str, String); 4]> {
    let line = value.lines().next()?.trim();
    if let Some(rest) = line.strip_prefix("npm install") {
      let pkgs = rest.trim_start();
      let suffix = if pkgs.is_empty() { String::new() } else { format!(" {pkgs}") };
      return Some([
        ("npm", format!("npm install{suffix}")),
        ("yarn", format!("yarn add{suffix}")),
        ("pnpm", format!("pnpm add{suffix}")),
        ("bun", format!("bun add{suffix}")),
      ]);
    }
    if let Some(rest) = line.strip_prefix("npx create-") {
      let rest = rest.trim();
      return Some([
        ("npm", format!("npx create-{rest}")),
        ("yarn", format!("yarn create {rest}")),
        ("pnpm", format!("pnpm create {rest}")),
        ("bun", format!("bunx --bun create-{rest}")),
      ]);
    }
    if let Some(rest) = line.strip_prefix("npx ") {
      let rest = rest.trim();
      return Some([
        ("npm", format!("npx {rest}")),
        ("yarn", format!("yarn dlx {rest}")),
        ("pnpm", format!("pnpm dlx {rest}")),
        ("bun", format!("bunx --bun {rest}")),
      ]);
    }
    None
  }
}

impl Visitor for Apply {
  fn visit_node(&mut self, node: &mut Node) -> NodeAction {
    let Node::CodeBlock(cb) = node else { return NodeAction::Keep };
    let Some(variants) = NpmCommand::derive(&cb.value) else { return NodeAction::Keep };
    let span = cb.span.clone();

    let pm_attrs: Vec<JsxAttr> = variants
      .iter()
      .map(|(name, value)| JsxAttr {
        name: name.to_string(),
        value: JsxAttrValue::String(value.clone()),
        span: span.clone(),
      })
      .collect();

    let theme_div = |mode: &str| -> Node {
      Node::JsxElement(JsxElement {
        name: "div".to_string(),
        attrs: vec![JsxAttr {
          name: "data-theme".to_string(),
          value: JsxAttrValue::String(mode.to_string()),
          span: span.clone(),
        }],
        children: vec![Node::JsxSelfClosing(JsxSelfClosing {
          name: "PackageManagerTabs".to_string(),
          attrs: pm_attrs.clone(),
          span: span.clone(),
        })],
        span: span.clone(),
      })
    };

    // Wrap matches velite's `rehype-pretty-code` fragment shape: an outer
    // `data-dmc-fragment` div with one `data-theme="<mode>"`
    // child per theme. Consumer CSS picks which copy to show by theme; the
    // tab content itself is theme-independent so both copies carry the
    // same `<PackageManagerTabs>` payload.
    let fragment = Node::JsxElement(JsxElement {
      name: "div".to_string(),
      attrs: vec![JsxAttr {
        // Empty-string value matches velite's `data-dmc-fragment=""`
        // serialization. A `JsxAttrValue::Boolean` would render as `="true"` for
        // this non-standard data-attr, breaking consumer CSS selectors that
        // target the empty form (e.g. `[data-dmc-fragment=""]`).
        name: "data-dmc-fragment".to_string(),
        value: JsxAttrValue::String(String::new()),
        span: span.clone(),
      }],
      children: vec![theme_div("dark"), theme_div("light")],
      span,
    });

    NodeAction::Replace(vec![fragment])
  }
}
