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
fn preserves_multiline_import_body_verbatim() {
  let src = "import {\n  A,\n  B,\n} from \"./{brace}.js\"\n# H\n";
  let d = parse_doc(src);
  let imp = d.children.iter().find_map(|n| match n {
    Node::Import(i) => Some(i),
    _ => None,
  });
  let imp = imp.expect("expected Import node");
  assert_eq!(imp.raw, "import {\n  A,\n  B,\n} from \"./{brace}.js\"");
}

#[test]
fn preserves_export_body_with_markdownish_strings() {
  let src = "export const z = { text: \"# not heading\", link: \"[x](y)\", braces: \"{}\" }\n# H\n";
  let d = parse_doc(src);
  let export = d.children.iter().find_map(|n| match n {
    Node::Export(e) => Some(e),
    _ => None,
  });
  let export = export.expect("expected Export node");
  assert_eq!(export.raw, "export const z = { text: \"# not heading\", link: \"[x](y)\", braces: \"{}\" }");
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
  // fm, imp, h in order (paragraphs may interleave).
  let positions: Vec<_> = ["fm", "imp", "h"].iter().map(|w| kinds.iter().position(|k| k == w)).collect();
  assert!(positions.iter().all(|p| p.is_some()), "got {:?}", kinds);
  assert!(positions[0] < positions[1]);
  assert!(positions[1] < positions[2]);
}
