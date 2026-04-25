use duck_md_ast::*;
use duck_md_parser::parse;
use duck_md_transform::{AutolinkHeadings, Pipeline};

#[test]
fn pipeline_runs_autolink() {
  let mut d = parse("# Hello");
  Pipeline::new().add(AutolinkHeadings::new()).run(&mut d);
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
  Pipeline::new().add(AutolinkHeadings::new()).run(&mut d);
  Pipeline::new().add(AutolinkHeadings::new()).run(&mut d);
  let h = match &d.children[0] {
    Node::Heading(h) => h,
    n => panic!("expected heading, got {:?}", n),
  };
  assert_eq!(h.children.len(), 1, "autolink should not double-wrap");
}

#[test]
fn defaults_pipeline_includes_autolink() {
  let mut d = parse("# Foo Bar");
  Pipeline::with_defaults().run(&mut d);
  let h = match &d.children[0] {
    Node::Heading(h) => h,
    n => panic!("expected heading, got {:?}", n),
  };
  assert!(matches!(h.children.first(), Some(Node::Link(_))));
}
