//! The keys a compile puts on the wire.
//!
//! `dmc-napi/mod.ts` hand-writes the TypeScript for this struct, and nothing links the two:
//! `tsc` is happy with an interface that omits a field, so a new field here reaches JS
//! callers as a value their types say does not exist. That is how `diagnostics` shipped
//! invisible in 0.6.0. Adding a field below is meant to fail this test, as the reminder to
//! describe it on the other side.

use dmc::engine::compile::{CompileConfig, Compiler};
use dmc_diagnostic::Code;
use duck_diagnostic::DiagnosticEngine;
use std::path::Path;

/// Every key, in the order `CompileOutput` declares them. The struct is `rename_all =
/// "camelCase"`, so these are the names a JS caller sees, not the Rust field names.
const KEYS: &[&str] = &[
  "frontmatter",
  "frontmatterRaw",
  "content",
  "html",
  "body",
  "excerpt",
  "metadata",
  "toc",
  "imports",
  "exports",
  "components",
];

#[test]
fn a_compile_serializes_the_keys_the_typescript_describes() {
  let mut diag = DiagnosticEngine::<Code>::new();
  let out = Compiler::compile_with_pipeline(
    "---\ntitle: T\n---\n\n# H\n\ntext\n",
    Path::new("."),
    &CompileConfig::new(),
    &mut diag,
  );
  let v = serde_json::to_value(&out).expect("serialize");
  let obj = v.as_object().expect("an object");

  let mut got: Vec<&str> = obj.keys().map(String::as_str).collect();
  let mut want: Vec<&str> = KEYS.to_vec();
  got.sort_unstable();
  want.sort_unstable();
  assert_eq!(
    got, want,
    "CompileOutput's keys changed. Update `CompileOutput` in dmc-napi/mod.ts to match, then this list."
  );
}

#[test]
fn components_is_always_on_the_wire_even_when_nothing_asked_for_it() {
  // `#[serde(default)]` on the field only affects reading a record back, so `components`
  // serializes on every compile. A caller that treats its presence as "this document has
  // components" is reading an empty array as a yes.
  let mut diag = DiagnosticEngine::<Code>::new();
  let out = Compiler::compile_with_pipeline("text\n", Path::new("."), &CompileConfig::new(), &mut diag);
  let v = serde_json::to_value(&out).expect("serialize");
  assert_eq!(v.get("components"), Some(&serde_json::json!([])), "present, and empty rather than absent");
}
