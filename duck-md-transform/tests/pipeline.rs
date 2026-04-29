use duck_md_parser::ast::*;
use duck_md_parser::parse;
use duck_md_transform::{AutolinkHeadings, CodeImport, Pipeline};

#[test]
fn pipeline_runs_autolink() {
  let mut d = parse("# Hello");
  Pipeline::new().add(AutolinkHeadings::new()).run_silent(&mut d);
  let h = match &d.children[0] {
    Node::Heading(h) => h,
    n => panic!("expected heading, got {:?}", n),
  };
  assert_eq!(h.children.len(), 1);
  match &h.children[0] {
    Node::Link(l) => {
      assert_eq!(l.href, "#hello");
      assert_eq!(l.title.as_deref(), Some("Link to section"));
    },
    n => panic!("expected Link wrap, got {:?}", n),
  }
}

#[test]
fn idempotent() {
  let mut d = parse("# Hello");
  Pipeline::new().add(AutolinkHeadings::new()).run_silent(&mut d);
  Pipeline::new().add(AutolinkHeadings::new()).run_silent(&mut d);
  let h = match &d.children[0] {
    Node::Heading(h) => h,
    n => panic!("expected heading, got {:?}", n),
  };
  assert_eq!(h.children.len(), 1, "autolink should not double-wrap");
}

#[test]
fn defaults_pipeline_includes_autolink() {
  let mut d = parse("# Foo Bar");
  Pipeline::with_defaults().run_silent(&mut d);
  let h = match &d.children[0] {
    Node::Heading(h) => h,
    n => panic!("expected heading, got {:?}", n),
  };
  assert!(matches!(h.children.first(), Some(Node::Link(_))));
}

// #[test]
// fn npm_command_derives_yarn_pnpm_bun() {
//   let mut d = duck_md_parser::parse("```\nnpm install lodash\n```\n");
//   duck_md_transform::Pipeline::new().add(NpmCommand).run_silent(&mut d);
//   let cb = d
//     .children
//     .iter()
//     .find_map(|n| match n {
//       Node::CodeBlock(cb) => Some(cb),
//       _ => None,
//     })
//     .expect("code block");
//   let c = cb.commands.as_ref().expect("commands");
//   assert_eq!(c.npm, "npm install lodash");
//   assert_eq!(c.yarn, "yarn add lodash");
//   assert_eq!(c.pnpm, "pnpm add lodash");
//   assert_eq!(c.bun, "bun add lodash");
// }
//
// #[test]
// fn npm_command_handles_npx_create() {
//   let mut d = duck_md_parser::parse("```\nnpx create-next-app my-app\n```\n");
//   duck_md_transform::Pipeline::new().add(NpmCommand).run_silent(&mut d);
//   let cb = d
//     .children
//     .iter()
//     .find_map(|n| match n {
//       Node::CodeBlock(cb) => Some(cb),
//       _ => None,
//     })
//     .expect("cb");
//   let c = cb.commands.as_ref().expect("c");
//   assert_eq!(c.bun, "bunx create-next-app my-app");
// }

#[test]
fn code_import_reads_file() {
  let dir = tempfile::tempdir().unwrap();
  let snippet = dir.path().join("snippet.ts");
  std::fs::write(&snippet, "export const x = 1\n").unwrap();
  let src = "```ts file=\"snippet.ts\"\nplaceholder\n```\n".to_string();
  let mut d = duck_md_parser::parse(&src);
  duck_md_transform::Pipeline::new()
    .add(CodeImport::with_base_dir(dir.path().to_path_buf()))
    .run_silent(&mut d);
  let cb = d
    .children
    .iter()
    .find_map(|n| match n {
      Node::CodeBlock(cb) => Some(cb),
      _ => None,
    })
    .expect("cb");
  assert!(cb.value.contains("export const x = 1"), "got {:?}", cb.value);
}
