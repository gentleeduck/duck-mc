mod common;
use common::*;
use duck_md_ast::*;
use pretty_assertions::assert_eq;

#[test]
fn empty_doc_has_zero_children() {
    let d = parse_doc("");
    assert!(d.children.is_empty(), "got {:?}", d.children);
}

#[test]
fn parses_h1() {
    let d = parse_doc("# Hello");
    assert_eq!(d.children.len(), 1);
    match &d.children[0] {
        Node::Heading(h) => {
            assert_eq!(h.level, 1);
            assert_eq!(h.id, "hello");
        }
        n => panic!("expected Heading, got {:?}", n),
    }
}

#[test]
fn parses_paragraph() {
    let d = parse_doc("hello world");
    assert!(matches!(d.children[0], Node::Paragraph(_)), "got {:?}", d.children);
}

#[test]
fn h2_followed_by_paragraph() {
    let d = parse_doc("## Title\nbody text");
    assert!(d.children.len() >= 2, "got {:?}", d.children);
    match &d.children[0] {
        Node::Heading(h) => assert_eq!(h.level, 2),
        n => panic!("expected Heading, got {:?}", n),
    }
}
