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

/// Regression: a nest of PascalCase JSX elements indented 2 / 4 / 6
/// spaces, carrying inline content with a lowercase HTML tag that
/// itself has a JSX-style attribute (`<code className="...">x</code>`),
/// must NOT silently fold its inner JSX children into an indented
/// `CodeBlock`. The fallback indented-code detection used to fire on
/// every indented line of a JSX container, swallowing the whole body
/// (and effectively dropping the inner components from the AST).
#[test]
fn lowercase_html_tag_with_jsx_attr_inline_does_not_drop_enclosing_jsx() {
  let mdx = "# H\n\n<Outer>\n  <Inner attr=\"a\">\n    <InnerInner className=\"b\">\n      No. <code className=\"x\">react</code>, more.\n    </InnerInner>\n  </Inner>\n</Outer>\n";
  let doc = dmc_parser::parse(mdx);
  let outer = first_jsx_element(&doc.children, "Outer").expect("<Outer> dropped from the AST");
  // All three nested JSX elements survive.
  let inner = first_jsx_element(&outer.children, "Inner").expect("<Inner> missing under <Outer>");
  let inner_inner = first_jsx_element(&inner.children, "InnerInner").expect("<InnerInner> missing under <Inner>");
  // The inner body must NOT have collapsed into an indented CodeBlock.
  fn collect_code_blocks(nodes: &[Node], out: &mut usize) {
    for n in nodes {
      if matches!(n, Node::CodeBlock(_)) {
        *out += 1;
      }
      collect_code_blocks(Node::children_of(n), out);
    }
  }
  let mut n_code = 0;
  collect_code_blocks(&inner_inner.children, &mut n_code);
  assert_eq!(n_code, 0, "<InnerInner> body should not contain a CodeBlock; got {:#?}", inner_inner.children);
  // The inline `<code className=\"x\">react</code>` round-trips as raw
  // HTML (CM 6.6) and carries the attribute / whitespace intact.
  let mut html_values = Vec::new();
  fn collect_html(nodes: &[Node], out: &mut Vec<String>) {
    for n in nodes {
      if let Node::Html(h) = n {
        out.push(h.value.clone());
      }
      collect_html(Node::children_of(n), out);
    }
  }
  collect_html(&inner_inner.children, &mut html_values);
  assert!(
    html_values.iter().any(|v| v == "<code className=\"x\">"),
    "inline `<code className=...>` should survive as raw HTML; got {:?}",
    html_values
  );
  assert!(
    html_values.iter().any(|v| v == "</code>"),
    "inline `</code>` close tag should survive as raw HTML; got {:?}",
    html_values
  );
}

/// Regression (UNINDENTED): the production preMdx pipeline re-serializes
/// MDX with everything at column 0 -- no indentation inside JSX blocks.
/// An `<AccordionContent>` whose body paragraph contains an inline
/// `<code className="...">react</code>` must not drop the enclosing
/// `<Accordion>` / `<AccordionItem>` / `<AccordionContent>` from the AST.
#[test]
fn unindented_lowercase_html_with_jsx_attr_inline_does_not_drop_enclosing_jsx() {
  let mdx = "# H\n\n<Accordion type=\"multiple\" collapsible className=\"w-full\">\n<AccordionItem value=\"x\">\n<AccordionTrigger>Q?</AccordionTrigger>\n\n<AccordionContent className=\"text-muted-foreground\">\nNo. <code className=\"rounded bg-muted px-2 py-1\">react</code>, more text.\n</AccordionContent>\n</AccordionItem>\n</Accordion>\n";
  let doc = dmc_parser::parse(mdx);
  let accordion = first_jsx_element(&doc.children, "Accordion").expect("<Accordion> dropped from the AST");
  let item =
    first_jsx_element(&accordion.children, "AccordionItem").expect("<AccordionItem> missing under <Accordion>");
  let _trigger =
    first_jsx_element(&item.children, "AccordionTrigger").expect("<AccordionTrigger> missing under <AccordionItem>");
  let content =
    first_jsx_element(&item.children, "AccordionContent").expect("<AccordionContent> missing under <AccordionItem>");
  // The inline `<code className="...">react</code>` round-trips as raw
  // HTML; the enclosing AccordionContent body still parses.
  fn collect_html(nodes: &[Node], out: &mut Vec<String>) {
    for n in nodes {
      if let Node::Html(h) = n {
        out.push(h.value.clone());
      }
      collect_html(Node::children_of(n), out);
    }
  }
  let mut html_values = Vec::new();
  collect_html(&content.children, &mut html_values);
  assert!(
    html_values.iter().any(|v| v == "<code className=\"rounded bg-muted px-2 py-1\">"),
    "inline `<code className=...>` should survive as raw HTML; got {:?}",
    html_values
  );
  assert!(
    html_values.iter().any(|v| v == "</code>"),
    "inline `</code>` close tag should survive as raw HTML; got {:?}",
    html_values
  );
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

/// A `<div className="...">` block with `<LinkedCard>` component
/// children compiles as nested JSX elements -- not one verbatim raw-HTML
/// blob (which would never instantiate the components and would render
/// `className` as an inert HTML attribute). The lowercase `<svg>` /
/// `<title>` / `<path>` / `<p>` descendants become JSX elements too, so
/// their attributes survive. Inter-element indentation / line breaks are
/// dropped (JSX ignores that whitespace).
#[test]
fn classed_div_with_component_children_parses_as_jsx() {
  let src = "\
<div className=\"mt-8 grid gap-4 sm:grid-cols-2\">
  <LinkedCard href=\"/a\">
    <svg viewBox=\"0 0 24 24\" className=\"h-10 w-10\" fill=\"currentColor\">
      <title>Next.js</title>
      <path d=\"M11\" />
    </svg>
    <p className=\"mt-2 font-medium\">Next.js</p>
  </LinkedCard>
  <LinkedCard href=\"/b\">
    <p className=\"mt-2 font-medium\">Vite</p>
  </LinkedCard>
</div>
";
  let d = parse_doc(src);
  let div = d
    .children
    .iter()
    .find_map(|n| match n {
      Node::JsxElement(e) if e.name == "div" => Some(e),
      _ => None,
    })
    .unwrap_or_else(|| panic!("expected a <div> JsxElement, got {:?}", d.children));
  assert!(div.attrs.iter().any(|a| a.name == "className"), "div className attr should survive, got {:?}", div.attrs);
  // Two LinkedCard children, in order, no stray whitespace text nodes.
  let cards: Vec<&JsxElement> = div
    .children
    .iter()
    .map(|n| match n {
      Node::JsxElement(e) if e.name == "LinkedCard" => e,
      other => panic!("unexpected non-LinkedCard child of <div>: {:?}", other),
    })
    .collect();
  assert_eq!(cards.len(), 2, "expected 2 LinkedCard children, got {:?}", div.children);
  // First card: <svg> element (with its <title>/<path> kids) + a <p>.
  let svg = cards[0]
    .children
    .iter()
    .find_map(|n| match n {
      Node::JsxElement(e) if e.name == "svg" => Some(e),
      _ => None,
    })
    .unwrap_or_else(|| panic!("expected an <svg> child of LinkedCard, got {:?}", cards[0].children));
  assert!(svg.attrs.iter().any(|a| a.name == "viewBox"), "svg viewBox should survive");
  assert!(
    svg.children.iter().any(|n| matches!(n, Node::JsxElement(e) if e.name == "title")),
    "svg should keep its <title> element child, got {:?}",
    svg.children
  );
  assert!(
    svg.children.iter().any(|n| matches!(n, Node::JsxSelfClosing(e) if e.name == "path")),
    "svg should keep its self-closing <path> child, got {:?}",
    svg.children
  );
  assert!(
    cards[0].children.iter().any(|n| matches!(n, Node::JsxElement(e) if e.name == "p"
      && e.attrs.iter().any(|a| a.name == "className"))),
    "first card should keep its <p className=...> child, got {:?}",
    cards[0].children
  );
}

/// A plain `<div>...</div>` with no JSX syntax and no component children
/// still routes through the CommonMark raw-HTML block path.
#[test]
fn plain_div_block_stays_raw_html() {
  let d = parse_doc("<div>\nhello\n</div>\n");
  assert!(
    d.children.iter().any(|n| matches!(n, Node::Html(_))),
    "plain <div> block should be a raw-HTML node, got {:?}",
    d.children
  );
  assert!(
    !d.children.iter().any(|n| matches!(n, Node::JsxElement(e) if e.name == "div")),
    "plain <div> block must not be a JsxElement, got {:?}",
    d.children
  );
}
