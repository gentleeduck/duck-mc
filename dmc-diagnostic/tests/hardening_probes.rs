//! Probes that every diagnostic code has a stable, non-empty string id
//! and a defined severity, and that the namespace prefix (`E`/`P`/`T`/
//! `G`/`C`/`S`) matches the layer feature gate.

use dmc_diagnostic::Code;
use duck_diagnostic::{DiagnosticCode, Severity};

fn assert_well_formed(c: &Code, expected_prefix: &[&str]) {
  let id = c.code();
  assert!(!id.is_empty(), "empty code id for {c:?}");
  assert!(id.len() >= 2, "code id too short for {c:?}: {id}");
  assert!(expected_prefix.iter().any(|p| id.starts_with(p)), "code {id} does not start with {expected_prefix:?}");
  let sev = c.severity();
  assert!(matches!(sev, Severity::Error | Severity::Warning), "unexpected severity for {c:?}: {sev:?}");
  // Clone preserves identity.
  let clone = c.clone();
  assert_eq!(clone.code(), id);
}

#[test]
fn lexer_codes_have_e_or_w_prefix() {
  let codes: &[Code] = &[
    Code::InvalidCharacter,
    Code::InvalidFrontMatter,
    Code::UnterminatedString,
    Code::UnterminatedExpression,
    Code::UnexpectedEof,
    Code::InvalidJsxSelfClosingTag,
    Code::UnterminatedJsxTag,
    Code::InvalidJsxClosingTag,
    Code::InvalidJsxAttribute,
    Code::UnterminatedCodeBlock,
    Code::EmptyFrontMatter,
  ];
  for c in codes {
    assert_well_formed(c, &["E", "W"]);
  }
}

#[test]
fn parser_codes_have_p_or_pw_prefix() {
  let codes: &[Code] = &[
    Code::UnterminatedLink,
    Code::UnterminatedImage,
    Code::UnterminatedInlineCode,
    Code::UnterminatedCodeBlockBlock,
    Code::UnterminatedJsxOpenTag,
    Code::UnterminatedJsxCloseTag,
    Code::UnterminatedJsxExpression,
    Code::UnterminatedMdComment,
    Code::UnterminatedFrontmatter,
    Code::MismatchedJsxCloseTag,
    Code::TableShapeMismatch,
    Code::StraySetextUnderline,
    Code::MissingJsxAttributeValue,
    Code::ListMarkerOverflow,
    Code::BlockNestingTooDeep,
    Code::EmptyFrontmatter,
    Code::InvalidFrontmatterYaml,
    Code::HeadingLevelClamped,
    Code::RecoveredUnterminatedJsx,
  ];
  for c in codes {
    assert_well_formed(c, &["P"]);
  }
}

#[test]
fn shared_codes_have_s_or_sw_prefix() {
  let codes: &[Code] = &[
    Code::IoRead,
    Code::IoWrite,
    Code::IoCreateDir,
    Code::JsonDeserialize,
    Code::JsonSerialize,
    Code::LockPoisoned,
    Code::IoRecoverable,
  ];
  for c in codes {
    assert_well_formed(c, &["S"]);
  }
}

#[test]
fn custom_code_passes_through_id_and_severity() {
  let c = Code::Custom { code: "X999".into(), severity: Severity::Error };
  assert_eq!(c.code(), "X999");
  assert!(matches!(c.severity(), Severity::Error));
}

#[test]
fn all_known_ids_are_unique() {
  let codes: Vec<Code> = vec![
    Code::InvalidCharacter,
    Code::InvalidFrontMatter,
    Code::UnterminatedString,
    Code::UnterminatedExpression,
    Code::UnexpectedEof,
    Code::InvalidJsxSelfClosingTag,
    Code::UnterminatedJsxTag,
    Code::InvalidJsxClosingTag,
    Code::InvalidJsxAttribute,
    Code::UnterminatedCodeBlock,
    Code::EmptyFrontMatter,
    Code::UnterminatedLink,
    Code::UnterminatedImage,
    Code::UnterminatedInlineCode,
    Code::UnterminatedCodeBlockBlock,
    Code::UnterminatedJsxOpenTag,
    Code::UnterminatedJsxCloseTag,
    Code::UnterminatedJsxExpression,
    Code::UnterminatedMdComment,
    Code::UnterminatedFrontmatter,
    Code::MismatchedJsxCloseTag,
    Code::TableShapeMismatch,
    Code::StraySetextUnderline,
    Code::MissingJsxAttributeValue,
    Code::ListMarkerOverflow,
    Code::BlockNestingTooDeep,
    Code::EmptyFrontmatter,
    Code::InvalidFrontmatterYaml,
    Code::HeadingLevelClamped,
    Code::RecoveredUnterminatedJsx,
    Code::IoRead,
    Code::IoWrite,
    Code::IoCreateDir,
    Code::JsonDeserialize,
    Code::JsonSerialize,
    Code::LockPoisoned,
    Code::IoRecoverable,
  ];
  use std::collections::HashSet;
  let mut seen: HashSet<&str> = HashSet::new();
  for c in &codes {
    let id = c.code();
    assert!(seen.insert(id), "duplicate code id: {id}");
  }
}
