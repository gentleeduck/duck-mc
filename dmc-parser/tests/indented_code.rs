use dmc_parser::ast::*;

mod common;
use common::parse_doc;

#[test]
fn indented_code_block() {
  let src = "para\n\n    fn main() {}\n\nafter\n";
  let doc = parse_doc(src);
  let has_code = doc.children.iter().any(|n| matches!(n, Node::CodeBlock(_)));
  assert!(has_code, "expected CodeBlock, got: {:#?}", doc.children);
}
