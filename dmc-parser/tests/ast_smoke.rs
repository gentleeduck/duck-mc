use dmc_parser::ast::*;
use pretty_assertions::assert_eq;

fn sp() -> duck_diagnostic::Span {
  default_span()
}

#[test]
fn build_a_simple_document() {
  let doc = Node::Document(Document {
    span: sp(),
    children: vec![
      Node::Heading(Heading {
        level: 1,
        span: sp(),
        children: vec![Node::Text(Text { value: "Hello".into(), span: sp() })],
      }),
      Node::Paragraph(Paragraph {
        span: sp(),
        children: vec![
          Node::Text(Text { value: "world ".into(), span: sp() }),
          Node::Bold(Inline { span: sp(), children: vec![Node::Text(Text { value: "yo".into(), span: sp() })] }),
        ],
      }),
    ],
  });

  let json = serde_json::to_string(&doc).unwrap();
  assert!(json.contains("\"Heading\""));
  assert!(json.contains("\"Hello\""));
  let back: Node = serde_json::from_str(&json).unwrap();
  assert_eq!(doc, back);
}

#[test]
fn jsx_attr_round_trip() {
  let a = JsxAttr { name: "color".into(), value: JsxAttrValue::String("red".into()), span: default_span() };
  let s = serde_json::to_string(&a).unwrap();
  let b: JsxAttr = serde_json::from_str(&s).unwrap();
  assert_eq!(a, b);
}
