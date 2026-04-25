use duck_md_codegen::render_html;
use duck_md_parser::parse;
use pretty_assertions::assert_eq;

fn html(src: &str) -> String { render_html(&parse(src)) }

#[test]
fn h1_with_id() {
    // codegen does NOT include autolink — that's the transformer's job.
    // Test the codegen alone here, separate from the pipeline.
    use duck_md_ast::*;
    let doc = Document {
        span: duck_md_ast::default_span(),
        children: vec![Node::Heading(Heading {
            level: 1,
            id: "hello".into(),
            children: vec![Node::Text(Text { value: "Hello".into(), span: duck_md_ast::default_span() })],
            span: duck_md_ast::default_span(),
        })],
    };
    let html = duck_md_codegen::render_html(&doc);
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
    // > may stay or be escaped — both fine, but text content should not contain raw ampersands
    assert!(!h.contains(" & "));
}
