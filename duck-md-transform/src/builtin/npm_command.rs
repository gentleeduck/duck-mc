use crate::pipeline::Transformer;
use crate::visit::{NodeAction, Visitor, walk_root};
use duck_md_diagnostic::Code;
use duck_md_diagnostic::metadata::SourceMeta;
use duck_md_parser::ast::*;

/// Inspect each fenced code block; if its first line matches `npm install …`,
/// `npx create-…`, or `npx …`, derive the equivalent yarn/pnpm/bun forms and
/// stash them on `CodeBlock.commands` so renderers can produce package-manager
/// tabs.
#[derive(Default)]
pub struct NpmCommand;

impl Transformer for NpmCommand {
  fn name(&self) -> &str {
    "npm-command"
  }

  fn transform(
    &self,
    doc: &mut Document,
    #[allow(unused_variables)] meta: &SourceMeta,
    #[allow(unused_variables)] engine: &mut duck_diagnostic::DiagnosticEngine<Code>,
  ) {
    let mut v = Apply;
    walk_root(&mut doc.children, &mut v);
  }
}

struct Apply;

impl NpmCommand {
  /// Map the first line of a code block (`npm install foo`, `npx create-x`,
  /// `npx tool`) to the equivalent yarn / pnpm / bun invocations.
  fn derive(value: &str) -> Option<[(&'static str, String); 4]> {
    let line = value.lines().next()?.trim();
    if let Some(rest) = line.strip_prefix("npm install") {
      let pkgs = rest.trim();
      return Some([
        ("npm", format!("npm install {}", pkgs)),
        ("yarn", format!("yarn add {}", pkgs)),
        ("pnpm", format!("pnpm add {}", pkgs)),
        ("bun", format!("bun add {}", pkgs)),
      ]);
    }
    if let Some(rest) = line.strip_prefix("npx create-") {
      let rest = rest.trim();
      return Some([
        ("npm", format!("npx create-{rest}")),
        ("yarn", format!("yarn create {rest}")),
        ("pnpm", format!("pnpm create {rest}")),
        ("bun", format!("bunx create-{rest}")),
      ]);
    }
    if let Some(rest) = line.strip_prefix("npx ") {
      let rest = rest.trim();
      return Some([
        ("npm", format!("npx {rest}")),
        ("yarn", format!("yarn run {rest}")),
        ("pnpm", format!("pnpm run {rest}")),
        ("bun", format!("bunx {rest}")),
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

    let attrs: Vec<JsxAttr> = variants
      .iter()
      .map(|(name, value)| JsxAttr {
        name: name.to_string(),
        value: JsxAttrValue::String(value.to_string()),
        span: span.clone(),
      })
      .collect();

    let jsx =
      Node::JsxSelfClosing(JsxSelfClosing { name: "PackageManagerTabs".to_string(), attrs, span });

    NodeAction::Replace(vec![jsx])
  }
}
