mod common;
use common::*;
use dmc_parser::ast::*;
use pretty_assertions::assert_eq;

fn first_paragraph(d: &Document) -> &Paragraph {
  for c in &d.children {
    if let Node::Paragraph(p) = c {
      return p;
    }
  }
  panic!("no paragraph in {:?}", d.children);
}

#[test]
fn footnote_ref_emits_node() {
  let d = parse_doc("see [^1] there\n");
  let p = first_paragraph(&d);
  let id = p
    .children
    .iter()
    .find_map(|n| match n {
      Node::FootnoteRef(f) => Some(f.id.clone()),
      _ => None,
    })
    .expect("footnote ref");
  assert_eq!(id, "1");
}

#[test]
fn footnote_def_emits_node() {
  let d = parse_doc("[^1]: body of note\n");
  let def = d
    .children
    .iter()
    .find_map(|n| match n {
      Node::FootnoteDef(f) => Some(f),
      _ => None,
    })
    .expect("footnote def");
  assert_eq!(def.id, "1");
  assert!(!def.children.is_empty());
}

#[test]
fn shortcut_reference_link_resolves() {
  // `[foo]` after `[foo]: /url "title"` should become Link(href=/url).
  let d = parse_doc("[foo]\n\n[foo]: /url \"title\"\n");
  let p = first_paragraph(&d);
  let link = p
    .children
    .iter()
    .find_map(|n| match n {
      Node::Link(l) => Some(l),
      _ => None,
    })
    .expect("link");
  assert_eq!(link.href, "/url");
  assert_eq!(link.title.as_deref(), Some("title"));
}

#[test]
fn full_reference_link_resolves() {
  // `[text][label]` resolves via label, displays text as the link body.
  let d = parse_doc("[click here][foo]\n\n[foo]: /url\n");
  let p = first_paragraph(&d);
  let link = p
    .children
    .iter()
    .find_map(|n| match n {
      Node::Link(l) => Some(l),
      _ => None,
    })
    .expect("link");
  assert_eq!(link.href, "/url");
}

#[test]
fn collapsed_reference_link_resolves() {
  // `[foo][]` collapses to label `foo`.
  let d = parse_doc("[foo][]\n\n[foo]: /url\n");
  let p = first_paragraph(&d);
  let link = p
    .children
    .iter()
    .find_map(|n| match n {
      Node::Link(l) => Some(l),
      _ => None,
    })
    .expect("link");
  assert_eq!(link.href, "/url");
}

#[test]
fn unresolved_reference_link_falls_back_to_text() {
  // Bare `[unknown]` with no matching def stays literal.
  let d = parse_doc("[unknown]\n");
  let p = first_paragraph(&d);
  assert!(!p.children.iter().any(|n| matches!(n, Node::Link(_))), "got {:?}", p.children);
}

#[test]
fn ref_label_normalization_is_case_insensitive() {
  // CM 4.7: labels match case-insensitively with collapsed whitespace.
  let d = parse_doc("[FOO]\n\n[foo]: /url\n");
  let p = first_paragraph(&d);
  let link = p
    .children
    .iter()
    .find_map(|n| match n {
      Node::Link(l) => Some(l),
      _ => None,
    })
    .expect("link");
  assert_eq!(link.href, "/url");
}

#[test]
fn nested_brackets_in_link_text() {
  // CM 6.3: outer brackets pair correctly even when the link text
  // contains its own `[..]` run. Inner `[b]` is plain text since no
  // `(` follows it; outer `[...](u)` becomes the link.
  let d = parse_doc("[a [b] c](https://x.dev)");
  let p = first_paragraph(&d);
  let link = p
    .children
    .iter()
    .find_map(|n| match n {
      Node::Link(l) => Some(l),
      _ => None,
    })
    .expect("link");
  assert_eq!(link.href, "https://x.dev");
  let text: String = link
    .children
    .iter()
    .map(|n| match n {
      Node::Text(t) => t.value.clone(),
      _ => String::new(),
    })
    .collect();
  assert!(text.contains("a"), "got {:?}", link.children);
  assert!(text.contains("b"), "got {:?}", link.children);
  assert!(text.contains("c"), "got {:?}", link.children);
}

#[test]
fn parses_bold() {
  let d = parse_doc("**hello**");
  let p = first_paragraph(&d);
  assert!(p.children.iter().any(|n| matches!(n, Node::Bold(_))), "got {:?}", p.children);
}

#[test]
fn parses_italic() {
  let d = parse_doc("_hi_");
  let p = first_paragraph(&d);
  assert!(p.children.iter().any(|n| matches!(n, Node::Italic(_))), "got {:?}", p.children);
}

#[test]
fn parses_inline_code() {
  let d = parse_doc("hi `code` bye");
  let p = first_paragraph(&d);
  assert!(p.children.iter().any(|n| matches!(n, Node::InlineCode(_))), "got {:?}", p.children);
}

#[test]
fn parses_link() {
  let d = parse_doc("[text](https://x.dev)");
  let p = first_paragraph(&d);
  let link = p
    .children
    .iter()
    .find_map(|n| match n {
      Node::Link(l) => Some(l),
      _ => None,
    })
    .expect("link");
  assert_eq!(link.href, "https://x.dev");
}

#[test]
fn parses_image() {
  let d = parse_doc("![alt](https://x.dev/a.png)");
  let p = first_paragraph(&d);
  let img = p
    .children
    .iter()
    .find_map(|n| match n {
      Node::Image(i) => Some(i),
      _ => None,
    })
    .expect("image");
  assert_eq!(img.src, "https://x.dev/a.png");
  assert_eq!(img.alt, "alt");
}

#[test]
fn parses_fenced_code_block() {
  let src = "```ts\nlet x = 1\n```\n";
  let d = parse_doc(src);
  let cb = d
    .children
    .iter()
    .find_map(|n| match n {
      Node::CodeBlock(c) => Some(c),
      _ => None,
    })
    .expect("code block");
  assert_eq!(cb.lang.as_deref(), Some("ts"));
  assert!(cb.value.contains("let x = 1"), "got {:?}", cb.value);
}

#[test]
fn multi_codepoint_named_entity_decodes() {
  let d = parse_doc("&ngE;\n");
  let p = first_paragraph(&d);
  let text: String = p
    .children
    .iter()
    .filter_map(|n| match n {
      Node::Text(t) => Some(t.value.clone()),
      _ => None,
    })
    .collect();
  assert_eq!(text, "\u{2267}\u{0338}");
}

#[test]
fn sharp_s_reference_labels_case_fold() {
  let d = parse_doc("[ẞ]\n\n[SS]: /url\n");
  let p = first_paragraph(&d);
  let link = p
    .children
    .iter()
    .find_map(|n| match n {
      Node::Link(l) => Some(l),
      _ => None,
    })
    .expect("link");
  assert_eq!(link.href, "/url");
}
