use dmc_parser::ast::*;
use dmc_transform::{BareUrlAutolink, Pipeline};

#[test]
fn rewrites_bare_url_in_paragraph() {
  let mut d = dmc_parser::parse("see https://example.com for info\n");
  Pipeline::new().add(BareUrlAutolink).run_silent(&mut d);
  let p = d
    .children
    .iter()
    .find_map(|n| match n {
      Node::Paragraph(p) => Some(p),
      _ => None,
    })
    .expect("paragraph");
  let has_link = p.children.iter().any(|n| matches!(n, Node::Link(l) if l.href == "https://example.com"));
  assert!(has_link, "got {:?}", p.children);
}

#[test]
fn does_not_rewrite_when_no_url() {
  let mut d = dmc_parser::parse("nothing here\n");
  Pipeline::new().add(BareUrlAutolink).run_silent(&mut d);
  let p = d
    .children
    .iter()
    .find_map(|n| match n {
      Node::Paragraph(p) => Some(p),
      _ => None,
    })
    .expect("paragraph");
  assert!(p.children.iter().all(|n| matches!(n, Node::Text(_))));
}

#[test]
fn bare_www_prefix_only_does_not_panic() {
  // Regression: a `www.` run with nothing (or only trailing punctuation /
  // junk) after the dot used to slice `url[4..]` past the trimmed string.
  for src in ["Visit www.\n", "see www.,\n", "www.\n\nwww.x\n", "a www. b www.c.d e\n"] {
    let mut d = dmc_parser::parse(src);
    Pipeline::new().add(BareUrlAutolink).run_silent(&mut d);
    // a real `www.x.d` autolink should still resolve
    if src.contains("www.c.d") {
      let linked = d.children.iter().any(|n| match n {
        Node::Paragraph(p) => p.children.iter().any(|c| matches!(c, Node::Link(l) if l.href == "http://www.c.d")),
        _ => false,
      });
      assert!(linked, "expected www.c.d to autolink in {src:?}");
    }
  }
}
