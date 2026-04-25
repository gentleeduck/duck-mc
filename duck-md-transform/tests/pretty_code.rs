use duck_md_ast::Node;
use duck_md_transform::{Pipeline, PrettyCode};

#[test]
fn pretty_code_highlights_rust() {
    let mut d = duck_md_parser::parse("```rust\nfn main() {}\n```\n");
    Pipeline::new().add(PrettyCode::default()).run(&mut d);
    let cb = d.children.iter().find_map(|n| match n {
        Node::CodeBlock(cb) => Some(cb), _ => None,
    }).expect("cb");
    let h = cb.highlighted_html.as_ref().expect("html");
    assert!(h.contains("<pre class=\"pretty-code\""), "got: {}", h);
    assert!(h.contains("data-lang=\"rust\""));
    assert!(h.contains("<span"), "expected highlighted spans, got: {}", h);
}

#[test]
fn pretty_code_handles_unknown_language() {
    let mut d = duck_md_parser::parse("```nosuchlang\nhello\n```\n");
    Pipeline::new().add(PrettyCode::default()).run(&mut d);
    let cb = d.children.iter().find_map(|n| match n {
        Node::CodeBlock(cb) => Some(cb), _ => None,
    }).expect("cb");
    assert!(cb.highlighted_html.is_some(), "should still emit something");
}

#[test]
fn pretty_code_idempotent() {
    let mut d = duck_md_parser::parse("```rust\nfn x() {}\n```\n");
    Pipeline::new().add(PrettyCode::default()).run(&mut d);
    Pipeline::new().add(PrettyCode::default()).run(&mut d);
    let cb = d.children.iter().find_map(|n| match n {
        Node::CodeBlock(cb) => Some(cb), _ => None,
    }).expect("cb");
    assert!(cb.highlighted_html.is_some());
}
