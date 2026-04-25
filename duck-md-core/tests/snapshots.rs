use duck_md::compile;

#[test]
fn snapshot_basic_doc() {
  let src = include_str!("../../tests/fixtures/velite-parity/skills.mdx");
  let out = compile(src);
  insta::assert_json_snapshot!("skills_compile", out, {
    ".body" => "[REDACTED_FACTORY_STRING]",
    ".html" => "[REDACTED_HTML]",
  });
}

#[test]
fn snapshot_simple_heading() {
  let out = compile("# Hello\n\nworld");
  insta::assert_json_snapshot!("simple_heading", out, {
    ".body" => "[REDACTED_FACTORY_STRING]",
  });
}
