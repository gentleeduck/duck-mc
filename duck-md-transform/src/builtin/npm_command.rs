use crate::pipeline::Transformer;
use crate::visit::{walk_mut, VisitFlow, Visitor};
use duck_md_ast::*;

#[derive(Default)]
pub struct NpmCommand;

impl Transformer for NpmCommand {
  fn name(&self) -> &str {
    "npm-command"
  }

  fn transform(&self, doc: &mut Document) {
    let mut v = Apply;
    for c in &mut doc.children {
      walk_mut(c, &mut v);
    }
  }
}

struct Apply;

impl Visitor for Apply {
  fn visit_node(&mut self, node: &mut Node) -> VisitFlow {
    if let Node::CodeBlock(cb) = node
      && let Some(cmd) = derive_commands(&cb.value)
    {
      cb.commands = Some(cmd);
    }
    VisitFlow::Continue
  }
}

fn derive_commands(value: &str) -> Option<NpmCommands> {
  let line = value.lines().next()?.trim();
  if let Some(rest) = line.strip_prefix("npm install") {
    let pkgs = rest.trim();
    return Some(NpmCommands {
      npm: format!("npm install {}", pkgs).trim().to_string(),
      yarn: format!("yarn add {}", pkgs).trim().to_string(),
      pnpm: format!("pnpm add {}", pkgs).trim().to_string(),
      bun: format!("bun add {}", pkgs).trim().to_string(),
    });
  }
  if let Some(rest) = line.strip_prefix("npx create-") {
    let rest = rest.trim();
    return Some(NpmCommands {
      npm: format!("npx create-{}", rest),
      yarn: format!("yarn create {}", rest),
      pnpm: format!("pnpm create {}", rest),
      bun: format!("bunx create-{}", rest),
    });
  }
  if let Some(rest) = line.strip_prefix("npx ") {
    let rest = rest.trim();
    return Some(NpmCommands {
      npm: format!("npx {}", rest),
      yarn: format!("yarn dlx {}", rest),
      pnpm: format!("pnpm dlx {}", rest),
      bun: format!("bunx {}", rest),
    });
  }
  None
}
