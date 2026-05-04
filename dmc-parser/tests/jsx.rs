mod common;
use common::*;
use dmc_parser::ast::*;
use pretty_assertions::assert_eq;

#[test]
fn self_closing() {
  let d = parse_doc("<Btn color=\"red\" />");
  let any = d.children.iter().any(|n| match n {
    Node::JsxSelfClosing(j) => j.name == "Btn",
    Node::Paragraph(p) => p.children.iter().any(|c| matches!(c, Node::JsxSelfClosing(j) if j.name == "Btn")),
    _ => false,
  });
  assert!(any, "got {:?}", d.children);
}

#[test]
fn element_with_text_children() {
  let d = parse_doc("<Card>hi</Card>");
  let found = d.children.iter().any(|n| match n {
    Node::JsxElement(e) => e.name == "Card",
    Node::Paragraph(p) => p.children.iter().any(|c| matches!(c, Node::JsxElement(e) if e.name == "Card")),
    _ => false,
  });
  assert!(found, "got {:?}", d.children);
}

#[test]
fn fragment() {
  let d = parse_doc("<>hi</>");
  let found = d.children.iter().any(|n| match n {
    Node::JsxFragment(_) => true,
    Node::Paragraph(p) => p.children.iter().any(|c| matches!(c, Node::JsxFragment(_))),
    _ => false,
  });
  assert!(found, "got {:?}", d.children);
}

#[test]
fn standalone_expression() {
  let d = parse_doc("hello {name} bye");
  let found = d.children.iter().any(|n| match n {
    Node::JsxExpression(_) => true,
    Node::Paragraph(p) => p.children.iter().any(|c| matches!(c, Node::JsxExpression(_))),
    _ => false,
  });
  assert!(found, "got {:?}", d.children);
}

#[test]
fn attrs_string_and_expression() {
  let d = parse_doc("<Btn a=\"x\" b={y} c />");
  let attrs = d
    .children
    .iter()
    .find_map(|n| match n {
      Node::JsxSelfClosing(j) => Some(j.attrs.clone()),
      Node::Paragraph(p) => p.children.iter().find_map(|c| match c {
        Node::JsxSelfClosing(j) => Some(j.attrs.clone()),
        _ => None,
      }),
      _ => None,
    })
    .expect("attrs");
  assert_eq!(attrs.len(), 3, "attrs: {:?}", attrs);
  assert_eq!(attrs[0].name, "a");
  assert_eq!(attrs[1].name, "b");
  assert_eq!(attrs[2].name, "c");
  assert!(matches!(attrs[2].value, JsxAttrValue::Boolean));
}

#[test]
fn nested_jsx() {
  let d = parse_doc("<Outer><Inner /></Outer>");
  let found = d.children.iter().any(|n| match n {
        Node::JsxElement(e) if e.name == "Outer" => {
            e.children.iter().any(|c| matches!(c, Node::Paragraph(p) if p.children.iter().any(|cc| matches!(cc, Node::JsxSelfClosing(j) if j.name == "Inner")))) ||
            e.children.iter().any(|c| matches!(c, Node::JsxSelfClosing(j) if j.name == "Inner"))
        }
        Node::Paragraph(p) => p.children.iter().any(|c| matches!(c, Node::JsxElement(e) if e.name == "Outer" && e.children.iter().any(|cc| matches!(cc, Node::JsxSelfClosing(j) if j.name == "Inner") || matches!(cc, Node::Paragraph(pp) if pp.children.iter().any(|x| matches!(x, Node::JsxSelfClosing(j) if j.name == "Inner")))))),
        _ => false,
    });
  assert!(found, "got {:?}", d.children);
}
