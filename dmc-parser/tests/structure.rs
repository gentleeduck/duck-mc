mod common;
use common::*;
use dmc_parser::ast::*;

#[test]
fn parses_frontmatter() {
  let src = "---\ntitle: Hello\nslug: x\n---\n# Body";
  let d = parse_doc(src);
  let fm = d.children.iter().find_map(|n| match n {
    Node::Frontmatter(f) => Some(f),
    _ => None,
  });
  let fm = fm.expect("expected Frontmatter node");
  assert!(fm.raw.contains("title: Hello"));
  assert!(fm.raw.contains("slug: x"));
}

#[test]
fn parses_import() {
  let src = "import { Button } from './x'\n# H";
  let d = parse_doc(src);
  let imp = d.children.iter().find_map(|n| match n {
    Node::Import(i) => Some(i),
    _ => None,
  });
  let imp = imp.expect("expected Import node");
  assert!(imp.raw.contains("Button"), "raw was {:?}", imp.raw);
}

#[test]
fn parses_export() {
  let src = "export const name = 'x'\n# H";
  let d = parse_doc(src);
  assert!(d.children.iter().any(|n| matches!(n, Node::Export(_))));
}

#[test]
fn frontmatter_then_import_then_heading() {
  let src = "---\ntitle: T\n---\nimport X from 'x'\n# H\n";
  let d = parse_doc(src);
  let kinds: Vec<&'static str> = d
    .children
    .iter()
    .map(|n| match n {
      Node::Frontmatter(_) => "fm",
      Node::Import(_) => "imp",
      Node::Heading(_) => "h",
      Node::Paragraph(_) => "p",
      _ => "?",
    })
    .collect();
  // expect at least fm, imp, h in that order (paragraphs allowed in between is fine)
  let positions: Vec<_> = ["fm", "imp", "h"].iter().map(|w| kinds.iter().position(|k| k == w)).collect();
  assert!(positions.iter().all(|p| p.is_some()), "got {:?}", kinds);
  assert!(positions[0] < positions[1]);
  assert!(positions[1] < positions[2]);
}
