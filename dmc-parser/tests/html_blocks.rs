//! Raw HTML block coverage (CommonMark 4.6 types 1 + 6) re-routed
//! through the parser when the lexer initially classified the open
//! tag as JSX.

mod common;
use common::*;
use dmc_parser::ast::*;

fn first_html(d: &Document) -> &Html {
  for c in &d.children {
    if let Node::Html(h) = c {
      return h;
    }
  }
  panic!("no html block in {:?}", d.children);
}

#[test]
fn type1_pre_block_captured_verbatim() {
  let d = parse_doc("<pre>let x = 1;\nlet y = 2;</pre>\n");
  let h = first_html(&d);
  assert!(h.value.starts_with("<pre>"));
  assert!(h.value.contains("let x = 1;"));
  assert!(h.value.contains("</pre>"));
}

#[test]
fn type1_script_closes_on_matching_tag() {
  // Inner JSX-looking content stays inside the type-1 block.
  let d = parse_doc("<script>const a = <div>not jsx</div>;</script>\n");
  let h = first_html(&d);
  assert!(h.value.starts_with("<script>"));
  assert!(h.value.ends_with("</script>"), "got {:?}", h.value);
}

#[test]
fn type6_div_closes_on_blank_line() {
  let d = parse_doc("<div class=\"hero\">\nbody\n\nparagraph\n");
  let h = first_html(&d);
  assert!(h.value.starts_with("<div"));
  assert!(h.value.contains("body"));
  // Blank line ended the block; the paragraph is a sibling.
  assert!(d.children.iter().any(|n| matches!(n, Node::Paragraph(_))));
}

#[test]
fn type6_table_tag_routes_to_html() {
  let d = parse_doc("<table>\n<tr><td>cell</td></tr>\n</table>\n");
  let h = first_html(&d);
  assert!(h.value.contains("<table>"));
  assert!(h.value.contains("</table>"));
}

#[test]
fn standalone_close_tag_is_preserved_verbatim() {
  let d = parse_doc("</div>\n");
  let h = first_html(&d);
  assert_eq!(h.value, "</div>\n");
}

#[test]
fn capital_tag_stays_jsx() {
  // MDX dialect: capital-name tags are JSX components, never HTML
  // blocks. Type-7 is intentionally not handled.
  let d = parse_doc("<Component>x</Component>\n");
  assert!(!d.children.iter().any(|n| matches!(n, Node::Html(_))));
}
