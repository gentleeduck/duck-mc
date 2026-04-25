mod common;
use common::*;
use duck_md_ast::*;

#[test]
fn parses_simple_table() {
    let src = "| a | b |\n|---|---|\n| 1 | 2 |\n";
    let d = parse_doc(src);
    let t = d
        .children
        .iter()
        .find_map(|n| match n {
            Node::Table(t) => Some(t),
            _ => None,
        })
        .expect("table");
    assert_eq!(t.children.len(), 2); // header + 1 body
    assert_eq!(t.children[0].cells.len(), 2);
}

#[test]
fn parses_table_alignments() {
    let src = "| a | b | c |\n|:--|--:|:-:|\n| 1 | 2 | 3 |\n";
    let d = parse_doc(src);
    let t = d
        .children
        .iter()
        .find_map(|n| match n {
            Node::Table(t) => Some(t),
            _ => None,
        })
        .expect("table");
    assert_eq!(
        t.align,
        vec![TableAlign::Left, TableAlign::Right, TableAlign::Center]
    );
}

#[test]
fn non_table_paragraph_with_pipe() {
    // single line with `|` and no alignment row should NOT be a table
    let d = parse_doc("a | b\n");
    assert!(!d.children.iter().any(|n| matches!(n, Node::Table(_))));
}
