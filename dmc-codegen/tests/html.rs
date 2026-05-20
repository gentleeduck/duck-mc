use dmc_codegen::{RenderOptions, render_html, render_html_with};
use dmc_parser::parse;
use pretty_assertions::assert_eq;

fn html(src: &str) -> String {
  render_html(&parse(src))
}

#[test]
fn h1_with_id() {
  // Codegen emits `id` only when AST carries one; `AssignHeadingIds`
  // populates `h.id`. Auto-slugging here would diverge from CM spec.
  use dmc_parser::ast::*;
  let doc = Document {
    span: dmc_parser::ast::default_span(),
    children: vec![Node::Heading(Heading {
      level: 1,
      children: vec![Node::Text(Text { value: "Hello".into(), span: dmc_parser::ast::default_span() })],
      span: dmc_parser::ast::default_span(),
      id: Some("hello".into()),
    })],
  };
  let html = dmc_codegen::render_html(&doc);
  assert_eq!(html, "<h1 id=\"hello\">Hello</h1>\n");
}

#[test]
fn h1_without_id_omits_attribute() {
  use dmc_parser::ast::*;
  let doc = Document {
    span: dmc_parser::ast::default_span(),
    children: vec![Node::Heading(Heading {
      level: 1,
      children: vec![Node::Text(Text { value: "Hi".into(), span: dmc_parser::ast::default_span() })],
      span: dmc_parser::ast::default_span(),
      id: None,
    })],
  };
  let html = dmc_codegen::render_html(&doc);
  assert_eq!(html, "<h1>Hi</h1>\n");
}

#[test]
fn paragraph_with_bold() {
  assert_eq!(html("**hi**"), "<p><strong>hi</strong></p>\n");
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
  assert!(h.contains("<pre><code class=\"language-ts\""), "got {}", h);
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
  // CM 0.31.2 spec uses XHTML self-closing `<hr />`.
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

#[test]
fn strikethrough_does_not_cross_blank_line() {
  let h = render_html(&dmc_parser::parse("This ~~has a\n\nnew paragraph~~.\n"));
  assert_eq!(h, "<p>This ~~has a</p>\n<p>new paragraph~~.</p>\n");
}

#[test]
fn gfm_email_autolink_keeps_underscore_in_local_part() {
  let doc = dmc_parser::parse_with(
    "a.b-c_d@a.b\n\na.b-c_d@a.b.\n\na.b-c_d@a.b-\n\na.b-c_d@a.b_\n",
    dmc_parser::ParseOptions { cm_strict_html_blocks: false, gfm_autolinks: true, legacy_gfm_emphasis: false },
  );
  let h = render_html(&doc);
  assert_eq!(
    h,
    "<p><a href=\"mailto:a.b-c_d@a.b\">a.b-c_d@a.b</a></p>\n\
<p><a href=\"mailto:a.b-c_d@a.b\">a.b-c_d@a.b</a>.</p>\n\
<p>a.b-c_d@a.b-</p>\n\
<p>a.b-c_d@a.b_</p>\n"
  );
}

// --- SEC-001: javascript:/data: URL XSS ---

#[test]
fn rejects_javascript_url_in_link() {
  let h = html("[x](javascript:alert(1))");
  assert!(!h.contains("javascript:"), "javascript: href leaked: {h}");
  assert!(h.contains("href=\"#\""), "expected safe fallback href: {h}");
}

#[test]
fn rejects_javascript_url_in_image() {
  let h = html("![x](javascript:alert(1))");
  assert!(!h.contains("javascript:"), "javascript: src leaked: {h}");
  assert!(h.contains("src=\"#\""), "expected safe fallback src: {h}");
}

#[test]
fn rejects_data_url_in_image() {
  let h = html("![x](data:text/html,<script>alert(1)</script>)");
  assert!(!h.contains("data:"), "data: src leaked: {h}");
}

#[test]
fn keeps_safe_urls() {
  let h = html("[x](https://example.com/y) ![z](/img/a.png)");
  assert!(h.contains("href=\"https://example.com/y\""), "{h}");
  assert!(h.contains("src=\"/img/a.png\""), "{h}");
}

// --- SEC-002: raw HTML passthrough XSS ---

#[test]
fn raw_html_block_omitted_in_safe_mode() {
  let h = html("<script>alert(1)</script>\n");
  assert!(!h.contains("<script>"), "raw <script> passed through: {h}");
}

#[test]
fn inline_raw_html_escaped_in_safe_mode() {
  let h = html("a <img src=x onerror=alert(1)> b");
  assert!(!h.contains("<img src=x"), "raw inline HTML passed through: {h}");
  assert!(h.contains("&lt;img"), "inline raw HTML not escaped: {h}");
}

#[test]
fn raw_html_passthrough_when_opted_in() {
  let doc = parse("<div class=\"ok\">hi</div>\n");
  let h = render_html_with(&doc, RenderOptions { allow_dangerous_html: true, ..Default::default() });
  assert!(h.contains("<div class=\"ok\">"), "opt-in passthrough failed: {h}");
}

#[test]
fn gfm_disallowed_raw_html_can_be_enabled() {
  let doc = dmc_parser::parse(
    "<strong> <title> <style> <em>\n\n<blockquote>\n  <xmp> is disallowed.  <XMP> is also disallowed.\n</blockquote>\n",
  );
  let h = render_html_with(
    &doc,
    RenderOptions { gfm_disallowed_raw_html: true, allow_dangerous_html: true },
  );
  assert_eq!(
    h,
    "<p><strong> &lt;title> &lt;style> <em></p>\n\
<blockquote>\n  &lt;xmp> is disallowed.  &lt;XMP> is also disallowed.\n</blockquote>\n"
  );
}
