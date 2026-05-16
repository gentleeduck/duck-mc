#![cfg(feature = "math")]

use dmc_transform::Math;

#[test]
fn yaml_frontmatter_dollar_runs_are_preserved() {
  let src = "---\ntitle: Dollar Variables\ndescription: Compare $subject.id, $resource.attributes.ownerId, $env.ip values.\n---\n\nBody.\n";
  let out = Math::preprocess_source(src);
  assert!(out.starts_with("---\ntitle: Dollar Variables\ndescription: Compare $subject.id, $resource.attributes.ownerId, $env.ip values.\n---\n"), "got:\n{}", out);
  assert!(!out.contains("MathMl"), "frontmatter `$` runs leaked into math rewrite:\n{}", out);
}

#[test]
fn toml_frontmatter_dollar_runs_are_preserved() {
  let src = "+++\ntitle = \"x\"\ndesc = \"$a $b\"\n+++\nBody.\n";
  let out = Math::preprocess_source(src);
  assert!(out.starts_with("+++\ntitle = \"x\"\ndesc = \"$a $b\"\n+++\n"), "got:\n{}", out);
  assert!(!out.contains("MathMl"), "got:\n{}", out);
}

#[test]
fn body_math_after_frontmatter_still_rewrites() {
  let src = "---\ntitle: x\n---\n\nInline $a + b$ math.\n";
  let out = Math::preprocess_source(src);
  assert!(out.contains("MathMl"), "body math should still be rewritten:\n{}", out);
}

#[test]
fn no_frontmatter_still_rewrites_body_math() {
  let src = "Inline $a + b$ math.\n";
  let out = Math::preprocess_source(src);
  assert!(out.contains("MathMl"), "got:\n{}", out);
}

#[test]
fn unterminated_frontmatter_falls_through() {
  let src = "---\nstill open $a$ math\nno closing fence";
  let out = Math::preprocess_source(src);
  assert!(out.contains("MathMl"), "no closing fence means no frontmatter region, math runs in source:\n{}", out);
}
