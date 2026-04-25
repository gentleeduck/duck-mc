use duck_md::{compile, CompileOutput};
use pretty_assertions::assert_eq;

#[test]
fn plain_paragraph_compiles() {
  let out: CompileOutput = compile("hello world");
  assert!(out.html.contains("<p>"));
  assert_eq!(out.metadata.word_count, 2);
  assert!(out.excerpt.contains("hello"));
  assert!(out.toc.is_empty());
  assert!(out.imports.is_empty());
}

#[test]
fn frontmatter_extracted() {
  let src = "---\ntitle: T\n---\n# H";
  let out = compile(src);
  assert_eq!(
    out.frontmatter.get("title").and_then(|v| v.as_str()),
    Some("T")
  );
  assert!(out.frontmatter_raw.contains("title: T"));
  assert!(out.content.starts_with("# H"));
}

#[test]
fn toc_nested() {
  let src = "# A\n## B\n## C\n### D\n";
  let out = compile(src);
  assert_eq!(out.toc.len(), 1);
  let a = &out.toc[0];
  assert_eq!(a.title, "A");
  assert_eq!(a.url, "#a");
  assert_eq!(a.items.len(), 2); // B, C
  let c = &a.items[1];
  assert_eq!(c.title, "C");
  assert_eq!(c.items.len(), 1); // D
}

#[test]
fn imports_collected() {
  let src = "import X from 'x'\nimport Y from 'y'\n# H";
  let out = compile(src);
  assert_eq!(out.imports.len(), 2);
}

#[test]
fn excerpt_truncates_long_text() {
  let body: String = "word ".repeat(100);
  let out = compile(&body);
  assert!(
    out.excerpt.chars().count() <= 261,
    "got {} chars",
    out.excerpt.chars().count()
  );
}
