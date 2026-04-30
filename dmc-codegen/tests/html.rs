use dmc_codegen::render_html;
use dmc_parser::parse;
use pretty_assertions::assert_eq;

fn html(src: &str) -> String {
  render_html(&parse(src))
}

#[test]
fn h1_with_id() {
  // codegen does NOT include autolink — that's the transformer's job.
  // Test the codegen alone here, separate from the pipeline.
  use dmc_parser::ast::*;
  let doc = Document {
    span: dmc_parser::ast::default_span(),
    children: vec![Node::Heading(Heading {
      level: 1,
      children: vec![Node::Text(Text {
        value: "Hello".into(),
        span: dmc_parser::ast::default_span(),
      })],
      span: dmc_parser::ast::default_span(),
    })],
  };
  let html = dmc_codegen::render_html(&doc);
  assert_eq!(html, "<h1 id=\"hello\">Hello</h1>");
}

#[test]
fn paragraph_with_bold() {
  assert_eq!(html("**hi**"), "<p><strong>hi</strong></p>");
}

#[test]
fn paragraph_with_italic() {
  assert!(html("_hi_").contains("<em>hi</em>"));
}

#[test]
fn link_renders() {
  assert!(html("[t](https://x)").contains("<a href=\"https://x\">t</a>"));
}

#[test]
fn image_renders() {
  let h = html("![a](https://x.png)");
  assert!(h.contains("<img"));
  assert!(h.contains("src=\"https://x.png\""));
  assert!(h.contains("alt=\"a\""));
}

#[test]
fn fenced_code_with_lang() {
  let src = "```ts\nlet x = 1\n```\n";
  let h = html(src);
  assert!(h.contains("<pre><code class=\"gentledmc-language-ts\""), "got {}", h);
  assert!(h.contains("let x = 1"), "got {}", h);
}

#[test]
fn jsx_self_closing_passthrough() {
  let h = html("<Btn color=\"red\" />");
  assert!(h.contains("<Btn color=\"red\" />"), "got {}", h);
}

#[test]
fn jsx_element_passthrough() {
  let h = html("<Card>hi</Card>");
  assert!(h.contains("<Card>"), "got {}", h);
  assert!(h.contains("</Card>"), "got {}", h);
}

#[test]
fn escape_text_special_chars() {
  let h = html("a & b < c > d");
  assert!(h.contains("&amp;"));
  assert!(h.contains("&lt;"));
  // > may stay or be escaped — both fine, but text content should not contain raw ampersands
  assert!(!h.contains(" & "));
}

#[test]
fn unordered_list_renders() {
  let h = render_html(&dmc_parser::parse("- a\n- b\n"));
  assert!(h.contains("<ul>"));
  assert!(h.contains("<li>"));
  assert!(h.matches("<li>").count() >= 2);
}

#[test]
fn ordered_list_with_start_renders() {
  let h = render_html(&dmc_parser::parse("5. e\n6. f\n"));
  assert!(h.contains("<ol start=\"5\""));
}

#[test]
fn thematic_break_html() {
  let h = dmc_codegen::render_html(&dmc_parser::parse("---\n"));
  assert!(h.contains("<hr />"), "got {}", h);
}

#[test]
fn blockquote_html() {
  let h = dmc_codegen::render_html(&dmc_parser::parse("> hi\n"));
  assert!(h.contains("<blockquote>"), "got {}", h);
}

#[test]
fn table_html_with_align() {
  let h = dmc_codegen::render_html(&dmc_parser::parse("| a | b |\n|:--|--:|\n| 1 | 2 |\n"));
  assert!(h.contains("<table>"));
  assert!(h.contains("<thead>"));
  assert!(h.contains("<tbody>"));
  assert!(h.contains("align=\"left\""));
  assert!(h.contains("align=\"right\""));
}

// pretty-code transformer removed — syntax highlighting now handled by
// shiki/rehype-pretty-code via the JS sidecar plugin pipeline.
