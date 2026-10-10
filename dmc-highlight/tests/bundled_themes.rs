//! What the bundle actually contains.
//!
//! An unbundled theme name is a warning and a fallback, not an error, so a wrong name in
//! a config renders in some other theme rather than failing the build. That makes the
//! advertised list part of the contract: `dmc-napi/mod.ts` names these for autocomplete
//! behind a `(string & {})` escape hatch, so a name it lists that is not here typechecks
//! and then quietly does not work.

use dmc_highlight::list_bundled_themes;

/// Every name `PrettyCodeBundledTheme` in `dmc-napi/mod.ts` offers. Kept here rather than
/// read from that file so this test needs no path into another crate's package layout.
const ADVERTISED: &[&str] = &[
  "1337",
  "ansi",
  "base16",
  "base16-256",
  "Catppuccin Frappe",
  "Catppuccin Latte",
  "Catppuccin Macchiato",
  "Catppuccin Mocha",
  "Coldark-Cold",
  "Coldark-Dark",
  "DarkNeon",
  "gruvbox-dark",
  "gruvbox-light",
  "Nord",
  "OneHalfDark",
  "OneHalfLight",
  "Solarized (dark)",
  "Solarized (light)",
  "tokyo-night",
  "TwoDark",
];

#[test]
fn every_advertised_theme_is_really_bundled() {
  let bundled = list_bundled_themes();
  let missing: Vec<_> = ADVERTISED.iter().filter(|a| !bundled.contains(*a)).collect();
  assert!(
    missing.is_empty(),
    "named in mod.ts but not bundled, so they typecheck and then fall back silently: {missing:?}"
  );
}

#[test]
fn every_bundled_theme_is_advertised() {
  let bundled = list_bundled_themes();
  let unlisted: Vec<_> = bundled.iter().filter(|b| !ADVERTISED.contains(b)).collect();
  assert!(unlisted.is_empty(), "bundled but absent from mod.ts, so nothing autocompletes them: {unlisted:?}");
}

#[test]
fn a_light_and_a_dark_theme_are_both_available() {
  // The pair is the point: one compile serving a light and a dark mode needs two
  // distinct bundled names, which is what multiThemeStrategy exists for.
  let bundled = list_bundled_themes();
  for pair in [("OneHalfLight", "OneHalfDark"), ("Catppuccin Latte", "Catppuccin Mocha")] {
    assert!(bundled.contains(&pair.0), "{} is not bundled", pair.0);
    assert!(bundled.contains(&pair.1), "{} is not bundled", pair.1);
  }
}
