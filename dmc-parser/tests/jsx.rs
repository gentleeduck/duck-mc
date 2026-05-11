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

/// Collect every `Node::Text` value found anywhere in the tree.
fn collect_text_values(nodes: &[Node], out: &mut Vec<String>) {
  for n in nodes {
    if let Node::Text(t) = n {
      out.push(t.value.clone());
    }
    collect_text_values(Node::children_of(n), out);
  }
}

fn first_jsx_element<'a>(nodes: &'a [Node], name: &str) -> Option<&'a JsxElement> {
  for n in nodes {
    if let Node::JsxElement(e) = n
      && e.name == name
    {
      return Some(e);
    }
    if let Some(found) = first_jsx_element(Node::children_of(n), name) {
      return Some(found);
    }
  }
  None
}

/// Regression: a `</B>` close tag inside a `<A>...</A>` body must close
/// the inner `<B>`, not leak as `["</", "B", ">"]` text nodes and cause
/// the following siblings to nest wrongly.
#[test]
fn enclosing_jsx_close_tag_not_swallowed_as_text() {
  let d = parse_doc("<A>\n  <B>x</B>\n  <B>y</B>\n</A>\n");
  let a = first_jsx_element(&d.children, "A").expect("element A");
  // Both <B> elements are children of <A> (possibly inside one wrapping
  // paragraph), and exactly two of them exist.
  let mut bs: Vec<&JsxElement> = Vec::new();
  fn gather_b<'a>(nodes: &'a [Node], out: &mut Vec<&'a JsxElement>) {
    for n in nodes {
      match n {
        Node::JsxElement(e) if e.name == "B" => out.push(e),
        Node::JsxElement(e) => gather_b(&e.children, out),
        Node::Paragraph(p) => gather_b(&p.children, out),
        _ => {},
      }
    }
  }
  gather_b(&a.children, &mut bs);
  assert_eq!(bs.len(), 2, "expected two <B> children, got {:?}", a.children);
  // First <B> flattens to a single Text("x").
  let mut first_b_text = Vec::new();
  collect_text_values(&bs[0].children, &mut first_b_text);
  assert_eq!(first_b_text, vec!["x".to_string()], "first <B> children: {:?}", bs[0].children);
  // No leaked close-tag fragments anywhere.
  let mut all_text = Vec::new();
  collect_text_values(&d.children, &mut all_text);
  for v in &all_text {
    assert!(v != "</" && v != ">" && v != "B", "leaked close-tag text fragment {:?} in {:?}", v, all_text);
  }
}

/// The original Tabs document from the bug report parses with proper
/// nesting: `<TabsTrigger value="cli">` has child text "CLI" and the
/// `value="manual"` trigger is its sibling, not its child.
#[test]
fn tabs_document_parses_with_correct_nesting() {
  let src = "<Tabs defaultValue=\"cli\">\n\n<TabsList>\n  <TabsTrigger value=\"cli\">CLI</TabsTrigger>\n  <TabsTrigger value=\"manual\">Manual</TabsTrigger>\n</TabsList>\n\n<TabsContent value=\"cli\">\n\ncontent\n\n</TabsContent>\n\n</Tabs>\n";
  let d = parse_doc(src);
  let list = first_jsx_element(&d.children, "TabsList").expect("TabsList element");
  let mut triggers: Vec<&JsxElement> = Vec::new();
  fn gather<'a>(nodes: &'a [Node], out: &mut Vec<&'a JsxElement>) {
    for n in nodes {
      match n {
        Node::JsxElement(e) if e.name == "TabsTrigger" => out.push(e),
        Node::JsxElement(e) => gather(&e.children, out),
        Node::Paragraph(p) => gather(&p.children, out),
        _ => {},
      }
    }
  }
  gather(&list.children, &mut triggers);
  assert_eq!(triggers.len(), 2, "expected 2 TabsTrigger siblings under TabsList, got {:?}", list.children);
  // Neither trigger nests the other.
  for t in &triggers {
    let mut nested = Vec::new();
    gather(&t.children, &mut nested);
    assert!(nested.is_empty(), "TabsTrigger should not nest another TabsTrigger: {:?}", t.children);
  }
  let mut first_text = Vec::new();
  collect_text_values(&triggers[0].children, &mut first_text);
  assert_eq!(first_text, vec!["CLI".to_string()], "first TabsTrigger text: {:?}", triggers[0].children);
  // No leaked `</TabsTrigger>` / `</TabsList>` text fragments.
  let mut all_text = Vec::new();
  collect_text_values(&d.children, &mut all_text);
  for v in &all_text {
    assert!(
      v != "</" && v != ">" && v != "TabsTrigger" && v != "TabsList",
      "leaked close-tag fragment {:?} in {:?}",
      v,
      all_text
    );
  }
}

/// No regression: a lowercase `<p>foo</p>` and a bare `</div>` inside a
/// paragraph still round-trip as raw HTML, not as terminators.
#[test]
fn lowercase_html_tags_still_raw_html() {
  let d = parse_doc("<p>foo</p>\n");
  // `<p>...</p>` becomes a raw-HTML block (Html node) -- it must not be
  // routed through the JSX element path.
  let has_html = d.children.iter().any(|n| matches!(n, Node::Html(_)));
  assert!(has_html, "lowercase <p> should be raw HTML, got {:?}", d.children);

  let d2 = parse_doc("text </div> more\n");
  // The `</div>` survives as raw HTML inside the paragraph.
  let mut found_div = false;
  fn walk(nodes: &[Node], found: &mut bool) {
    for n in nodes {
      match n {
        Node::Html(h) if h.value.contains("</div>") => *found = true,
        Node::Paragraph(p) => walk(&p.children, found),
        _ => {},
      }
    }
  }
  walk(&d2.children, &mut found_div);
  assert!(found_div, "bare </div> should stay raw HTML, got {:?}", d2.children);
}
