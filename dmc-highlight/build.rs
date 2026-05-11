//! Scan `assets/themes-bat/*.tmTheme` + `assets/grammars-sublime/*.sublime-syntax`
//! at build time and emit two enums (`Theme`, `Grammar`) with one variant per
//! file, plus a `name()` accessor that returns the original file-stem name
//! (the key syntect uses to look up the theme / grammar at runtime).
//!
//! Output: `$OUT_DIR/assets_gen.rs`. Included by `dmc-codegen::highlight`.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
  let crate_root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
  let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

  let themes = scan(&crate_root.join("assets/themes-bat"), "tmTheme");
  let grammars = scan(&crate_root.join("assets/grammars-sublime"), "sublime-syntax");

  let mut out = String::new();
  emit_enum(&mut out, "Theme", "THEMES", &themes);
  emit_enum(&mut out, "Grammar", "GRAMMARS", &grammars);

  fs::write(out_dir.join("assets_gen.rs"), out).expect("write assets_gen.rs");

  // Re-run when assets change.
  println!("cargo:rerun-if-changed=assets/themes-bat");
  println!("cargo:rerun-if-changed=assets/grammars-sublime");
}

/// Collect file stems for every file with `ext` in `dir`. Sorted, deduped.
fn scan(dir: &Path, ext: &str) -> Vec<String> {
  let mut out = Vec::new();
  if !dir.exists() {
    return out;
  }
  for entry in fs::read_dir(dir).expect("read_dir").flatten() {
    let path = entry.path();
    if path.extension().and_then(|s| s.to_str()) != Some(ext) {
      continue;
    }
    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
      out.push(stem.to_string());
    }
  }
  out.sort();
  out.dedup();
  out
}

fn emit_enum(out: &mut String, ty_name: &str, all_const: &str, names: &[String]) {
  // De-dup by sanitized identifier; first wins. Multiple grammars/themes
  // can share a basename collision (e.g., "C" + "c") -> only first kept
  // as an enum variant. Both still load at runtime; only enum exposure
  // is affected.
  let mut variants: Vec<(String, &String)> = Vec::new();
  let mut seen = std::collections::HashSet::new();
  for n in names {
    let ident = sanitize_ident(n);
    if seen.insert(ident.clone()) {
      variants.push((ident, n));
    }
  }

  out.push_str(&format!(
    "/// Auto-generated at build time from the bundled assets.\n\
     /// Do not edit by hand; regenerate by changing files under\n\
     /// `assets/themes-bat/` or `assets/grammars-sublime/`.\n\
     #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]\n\
     #[allow(non_camel_case_types)]\n\
     pub enum {ty_name} {{\n",
  ));
  for (ident, _) in &variants {
    out.push_str(&format!("  {ident},\n"));
  }
  out.push_str("}\n\n");

  // name() -> &'static str
  out.push_str(&format!("impl {ty_name} {{\n"));
  out.push_str("  /// Returns the canonical syntect name (the file stem).\n");
  out.push_str("  pub const fn name(self) -> &'static str {\n");
  out.push_str("    match self {\n");
  for (ident, raw) in &variants {
    out.push_str(&format!("      Self::{ident} => {:?},\n", raw));
  }
  out.push_str("    }\n");
  out.push_str("  }\n");

  // from_str: name -> Option<Self>
  out.push_str(
    "  /// Inverse of [`Self::name`]: lookup an enum variant by its canonical name.\n\
     /// Returns `None` if no variant matches.\n\
     pub fn from_name(s: &str) -> Option<Self> {\n\
        match s {\n",
  );
  for (ident, raw) in &variants {
    out.push_str(&format!("      {:?} => Some(Self::{ident}),\n", raw));
  }
  out.push_str("      _ => None,\n");
  out.push_str("    }\n");
  out.push_str("  }\n");
  out.push_str("}\n\n");

  // ALL slice
  out.push_str(&format!(
    "/// Every variant in declaration order; useful for iteration in UIs.\n\
     pub const {all_const}: &[{ty_name}] = &[\n",
  ));
  for (ident, _) in &variants {
    out.push_str(&format!("  {ty_name}::{ident},\n"));
  }
  out.push_str("];\n\n");
}

/// Convert any string to a valid Rust identifier in PascalCase. Digits at the
/// start get an `N` prefix to keep the result a legal identifier.
fn sanitize_ident(s: &str) -> String {
  let mut out = String::with_capacity(s.len());
  let mut upper_next = true;
  for ch in s.chars() {
    if ch.is_ascii_alphanumeric() {
      if upper_next {
        out.push(ch.to_ascii_uppercase());
        upper_next = false;
      } else {
        out.push(ch);
      }
    } else {
      upper_next = true;
    }
  }
  if out.is_empty() {
    return "Unknown".into();
  }
  if out.chars().next().unwrap().is_ascii_digit() {
    let mut prefixed = String::with_capacity(out.len() + 1);
    prefixed.push('N');
    prefixed.push_str(&out);
    return prefixed;
  }
  out
}
