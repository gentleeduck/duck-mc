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

#[test]
fn snapshot_url_autolink() {
  let out = compile("see <https://rust-lang.org> here");
  insta::assert_json_snapshot!("url_autolink", out, {
    ".body" => "[REDACTED_FACTORY_STRING]",
  });
}

#[test]
fn snapshot_email_autolink() {
  let out = compile("contact <hi@example.com>");
  insta::assert_json_snapshot!("email_autolink", out, {
    ".body" => "[REDACTED_FACTORY_STRING]",
  });
}

#[test]
fn snapshot_indented_code() {
  let out = compile("para\n\n    fn main() {}\n\nafter\n");
  insta::assert_json_snapshot!("indented_code", out, {
    ".body" => "[REDACTED_FACTORY_STRING]",
    ".html" => "[REDACTED_HTML]",
  });
}

#[test]
fn snapshot_gfm_table() {
  let src = "| Lang | Year |\n| :--- | ---: |\n| Rust | 2010 |\n";
  let out = compile(src);
  insta::assert_json_snapshot!("gfm_table", out, {
    ".body" => "[REDACTED_FACTORY_STRING]",
    ".html" => "[REDACTED_HTML]",
  });
}

#[test]
fn snapshot_task_list() {
  let out = compile("- [x] done\n- [ ] open\n");
  insta::assert_json_snapshot!("task_list", out, {
    ".body" => "[REDACTED_FACTORY_STRING]",
  });
}

#[test]
fn snapshot_blockquote_w_strike() {
  let out = compile("> Block ~~old~~ new\n> next line\n");
  insta::assert_json_snapshot!("blockquote_strike", out, {
    ".body" => "[REDACTED_FACTORY_STRING]",
  });
}

#[test]
fn snapshot_jsx_passthrough() {
  let src = "import { Callout } from './x'\n\n<Callout type=\"info\">Hi **bold**</Callout>\n";
  let out = compile(src);
  insta::assert_json_snapshot!("jsx_passthrough", out, {
    ".body" => "[REDACTED_FACTORY_STRING]",
  });
}
