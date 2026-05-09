//! User-facing walkthrough: ../../dmc-docs/dmc-diagnostic/
//! Run `cargo doc --open -p dmc-diagnostic` for the inline rustdoc.

//! Unified diagnostic codes for the dmc pipeline.
//!
//! Every layer (lexer, parser, transform, codegen) emits into one shared
//! `DiagnosticEngine<Code>`. Per-layer variants are gated behind cargo
//! features so a crate that only needs lexer codes can opt out of the rest.
//!
//! ## Feature flags
//! - `lexer`     - `E***` lexer-emitted variants
//! - `parser`    - `P***` / `PW***` parser-emitted variants
//! - `transform` - `T***` / `TW***` transform-emitted variants
//! - `codegen`   - `G***` / `GW***` codegen-emitted variants
//!
//! A normal full build (e.g. via `dmc-core`) enables all features.

use duck_diagnostic::{Diagnostic, DiagnosticCode, Severity};
use serde::{Deserialize, Serialize};

pub mod metadata;

/// Canonical fallible-return type across the dmc pipeline.
///
/// `DiagResult<T>` = `Result<T, Diagnostic<Code>>`. Replaces the
/// `Result<_, std::io::Error>` / `Result<_, String>` / `Result<(), ()>`
/// patterns scattered across the workspace so every error path lands
/// in the same shape: a typed `Code`, a human message, optional
/// labels / help. Callers handle errors uniformly via `?`,
/// `engine.emit(d)`, or both.
///
/// Default `T = ()` for the common "did this side-effect succeed?"
/// signature.
///
/// Cost: type alias only. Zero runtime overhead vs. a hand-written
/// `Result<T, Diagnostic<Code>>`. Identical layout in monomorphised
/// code (same machine code, same drop semantics).
///
/// Convention to avoid double-emit:
/// - functions that PRODUCE a diagnostic and want the caller to
///   decide its fate return `DiagResult<T>`.
/// - functions that handle errors locally + emit into a passed-in
///   `&mut DiagnosticEngine<Code>` return plain `Result<T, ()>`
///   (or no result at all).
/// Mix the two and you get the same diagnostic in the engine twice.
pub type DiagResult<T = ()> = Result<T, Diagnostic<Code>>;

/// Stable, machine-readable diagnostic identifiers spanning the whole
/// pipeline. Codes use disjoint string namespaces per layer:
///
/// - `E***`  - lexer errors  (feature `lexer`)
/// - `W***`  - lexer warnings (feature `lexer`)
/// - `P***`  - parser errors  (feature `parser`)
/// - `PW***` - parser warnings (feature `parser`)
/// - `T***`  - transform errors  (feature `transform`)
/// - `TW***` - transform warnings (feature `transform`)
/// - `G***`  - codegen errors  (feature `codegen`)
/// - `GW***` - codegen warnings (feature `codegen`)
/// - `C***`  - core / engine errors  (feature `core`)
/// - `CW***` - core / engine warnings (feature `core`)
/// - `S***`  - shared cross-cutting errors (IO, JSON, locks; always available)
/// - `SW***` - shared cross-cutting warnings (always available)
///
/// `Custom { code, severity }` is the escape hatch for third-party
/// transformers that want to emit through the same engine without forking
/// this enum.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Code {
  // ===================================================================
  // Lexer - feature = "lexer"
  // ===================================================================
  /// E001 - Source byte the dispatcher cannot map to any token rule.
  #[cfg(feature = "lexer")]
  InvalidCharacter,
  /// E002 - Frontmatter `---` opened but inner YAML is malformed.
  #[cfg(feature = "lexer")]
  InvalidFrontMatter,
  /// E003 - Quoted string literal opened without a closer before EOL/EOF.
  #[cfg(feature = "lexer")]
  UnterminatedString,
  /// E004 - `{ ... }` expression opened but brace depth never returned to zero.
  #[cfg(feature = "lexer")]
  UnterminatedExpression,
  /// E005 - EOF reached mid-construct where more input was required.
  #[cfg(feature = "lexer")]
  UnexpectedEof,
  /// E006 - `<Tag /` seen but the closing `>` is missing.
  #[cfg(feature = "lexer")]
  InvalidJsxSelfClosingTag,
  /// E007 - `<Tag ...` open tag never reached `>` or `/>` before a hard break/EOF.
  #[cfg(feature = "lexer")]
  UnterminatedJsxTag,
  /// E008 - `</Tag` close tag malformed: missing name or `>`.
  #[cfg(feature = "lexer")]
  InvalidJsxClosingTag,
  /// E009 - JSX attribute `name=` had no following value (string / `{expr}`).
  #[cfg(feature = "lexer")]
  InvalidJsxAttribute,
  /// E010 - Fenced code block opened without an equal-length closer before EOF.
  #[cfg(feature = "lexer")]
  UnterminatedCodeBlock,

  /// W001 - Frontmatter parsed cleanly but YAML body was empty.
  #[cfg(feature = "lexer")]
  EmptyFrontMatter,

  // ===================================================================
  // Parser - feature = "parser"
  // ===================================================================
  /// P001 - `[text](href)` opened but `]` never seen before a hard break/EOF.
  #[cfg(feature = "parser")]
  UnterminatedLink,
  /// P002 - `![alt](src)` opened but `]` never seen before a hard break/EOF.
  #[cfg(feature = "parser")]
  UnterminatedImage,
  /// P003 - Backtick run inline never closes on the same line.
  #[cfg(feature = "parser")]
  UnterminatedInlineCode,
  /// P004 - Fenced code block opened but matching ` ``` ` (or longer) never seen.
  #[cfg(feature = "parser")]
  UnterminatedCodeBlockBlock,
  /// P005 - `<Tag ...` opened but no `>` / `/>` before the next block break.
  #[cfg(feature = "parser")]
  UnterminatedJsxOpenTag,
  /// P006 - `</Tag` opened but no `>` before the next block break.
  #[cfg(feature = "parser")]
  UnterminatedJsxCloseTag,
  /// P007 - `{ ... }` expression opened but no closing `}` at matching depth.
  #[cfg(feature = "parser")]
  UnterminatedJsxExpression,
  /// P008 - `{/* ... */}` markdown comment opened but no `*/}` before EOF.
  #[cfg(feature = "parser")]
  UnterminatedMdComment,
  /// P009 - Frontmatter `---` opened but no closing `---` line found.
  #[cfg(feature = "parser")]
  UnterminatedFrontmatter,
  /// P010 - `<Foo>` close-tag name does not match the most recent open tag.
  #[cfg(feature = "parser")]
  MismatchedJsxCloseTag,
  /// P011 - Table header line had N cells but alignment row had M (M != N).
  #[cfg(feature = "parser")]
  TableShapeMismatch,
  /// P012 - Setext underline `===` / `---` appeared without a preceding paragraph.
  #[cfg(feature = "parser")]
  StraySetextUnderline,
  /// P013 - JSX attribute appeared with `=` but no value (string / `{expr}`).
  #[cfg(feature = "parser")]
  MissingJsxAttributeValue,
  /// P014 - List item used an ordered marker number that overflows `u32`.
  #[cfg(feature = "parser")]
  ListMarkerOverflow,

  /// PW001 - Frontmatter parsed but YAML content was empty.
  #[cfg(feature = "parser")]
  EmptyFrontmatter,
  /// PW002 - YAML in frontmatter failed to parse; recovered by treating as null.
  #[cfg(feature = "parser")]
  InvalidFrontmatterYaml,
  /// PW003 - Heading level > 6 was clamped to 6.
  #[cfg(feature = "parser")]
  HeadingLevelClamped,
  /// PW004 - Auto-recovery synthesised a self-close for `<Tag ...` to keep parsing.
  #[cfg(feature = "parser")]
  RecoveredUnterminatedJsx,

  // ===================================================================
  // Transform - feature = "transform"
  // ===================================================================
  /// T001 - `CodeImport`: `file=path` referenced a path that could not be read.
  #[cfg(feature = "transform")]
  ImportFileNotFound,
  /// T002 - `CodeImport`: `{ranges}` spec was malformed.
  #[cfg(feature = "transform")]
  InvalidLineRange,
  /// T003 - `ComponentPreview`: `registry_index` JSON file failed to read.
  #[cfg(feature = "transform")]
  RegistryIndexUnreadable,
  /// T004 - `ComponentPreview`: `registry_index` content was not valid JSON.
  #[cfg(feature = "transform")]
  RegistryIndexMalformed,
  /// T005 - `ComponentPreview`: requested `name` not found in the registry index.
  #[cfg(feature = "transform")]
  RegistryEntryNotFound,
  /// T006 - `ComponentPreview`: registry entry's first file path could not be read.
  #[cfg(feature = "transform")]
  RegistrySourceUnreadable,
  /// T007 - `ComponentSource`: `path=` attribute pointed to an unreadable file.
  #[cfg(feature = "transform")]
  ComponentSourceUnreadable,
  /// T008 - `CopyLinkedFiles`: write to `assets_dir` failed mid-publish.
  #[cfg(feature = "transform")]
  AssetCopyFailed,
  /// T009 - `Mermaid`: `mmdc` exited non-zero or produced no SVG.
  #[cfg(feature = "transform")]
  MermaidRenderFailed,

  /// TW001 - `Mermaid`: `mmdc` CLI is not on PATH; the transformer becomes a no-op.
  #[cfg(feature = "transform")]
  MmdcUnavailable,
  /// TW002 - `ComponentPreview` / `ComponentSource`: required `name` / `path` attribute is missing.
  #[cfg(feature = "transform")]
  MissingComponentAttr,
  /// TW003 - `CopyLinkedFiles`: a referenced asset path did not exist; original `src` / `href` preserved.
  #[cfg(feature = "transform")]
  AssetSourceMissing,
  /// TW004 - `CodeImport` / `ComponentSource`: non-disk source (`Origin::Stdin` /
  /// `Inline` / `Memory`) without an explicit `base_dir`, so relative `file=` /
  /// `path=` paths can't be resolved.
  #[cfg(feature = "transform")]
  BaseDirNotFound,

  /// TW006 - `Math` (KaTeX): `katex::Opts::builder().build()` failed; the
  /// resulting renderer falls back to a no-op rendering for the affected
  /// span. Almost always a sign of a broken build (the args are constants).
  #[cfg(feature = "transform")]
  KatexOpts,
  /// TW005 - `PrettyCode`: a configured theme name is not present in the
  /// bundled syntect themes. Highlight falls back to the first bundled theme,
  /// so the missing mode silently produces wrong colors. The diagnostic
  /// lists every bundled theme so consumers can pick a valid one.
  #[cfg(feature = "transform")]
  ThemeNotBundled,

  // ===================================================================
  // Codegen - feature = "codegen"
  // ===================================================================
  /// G001 - Codegen encountered a JSX tag with an empty / invalid name.
  #[cfg(feature = "codegen")]
  MalformedJsxTagName,

  /// GW001 - `MdxBodyEmitter`: GFM `Table` node dropped (no inline table renderer
  /// yet). Run `disable-gfm` first to convert tables to plain text.
  #[cfg(feature = "codegen")]
  MdxTableUnsupported,
  /// GW002 - `HtmlEmitter`: raw `JsxExpression` discarded (HTML output can't run JS);
  /// use the MDX body emitter for full JSX support.
  #[cfg(feature = "codegen")]
  HtmlExpressionDropped,

  // ===================================================================
  // Core - feature = "core"
  //
  // Engine-level codes use the `C***` / `CW***` namespace so they
  // never collide with the lexer's `E***` / `W***` strings (Cargo
  // unifies features across the workspace; defaults enable every
  // layer at once, so namespaces have to be globally unique).
  // ===================================================================
  /// C001 - No root dir configured.
  #[cfg(feature = "core")]
  NoRootDir,
  /// C002 - No config file found.
  #[cfg(feature = "core")]
  NoConfig,
  /// C003 - No collections configured.
  #[cfg(feature = "core")]
  NoCollections,
  /// C004 - Collection not found.
  #[cfg(feature = "core")]
  CollectionNotFound,
  /// C005 - Collection pattern not found.
  #[cfg(feature = "core")]
  CollectionPatternNotFound,
  /// C006 - Collection schema not found.
  #[cfg(feature = "core")]
  CollectionSchemaNotFound,
  /// C007 - Invalid config.
  #[cfg(feature = "core")]
  InvalidConfig,
  /// C008 - Invalid config path.
  #[cfg(feature = "core")]
  InvalidConfigPath,
  /// CW001 - Config file already exists at the target path.
  #[cfg(feature = "core")]
  ConfigExists,

  // ===================================================================
  // Shared (no feature gate)
  //
  // Cross-cutting concerns every layer hits: filesystem IO, JSON
  // round-trip, mutex poisoning. NOT gated behind a per-layer feature
  // because any crate may produce these (math cache load/save, sidecar
  // dispatch, engine output write, registry-index parse, ...). Using
  // them avoids leaking layer-specific codes (e.g. `EmptyFrontMatter`)
  // into IO failures unrelated to frontmatter.
  //
  // Namespace: `S***` for errors, `SW***` for warnings.
  // ===================================================================
  /// S001 - `std::fs::read*` / `read_to_string` failed at the named path.
  IoRead,
  /// S002 - `std::fs::write` failed at the named path.
  IoWrite,
  /// S003 - `std::fs::create_dir_all` failed for the named path.
  IoCreateDir,
  /// S004 - `serde_json` (or other deserializer) failed to parse the input.
  JsonDeserialize,
  /// S005 - `serde_json` (or other serializer) failed to encode the value.
  JsonSerialize,
  /// S006 - A `Mutex` / `RwLock` was poisoned by a panic in another thread.
  LockPoisoned,
  /// SW001 - Best-effort recoverable IO miss (e.g. cache load fell through).
  /// Build continues without the cached state.
  IoRecoverable,

  // ===================================================================
  // User-defined escape hatch - always available
  // ===================================================================
  /// Carry an arbitrary code string + explicit severity through the same
  /// engine. For third-party transformer authors who don't want to fork this
  /// enum. Prefer adding a typed variant when contributing upstream.
  Custom { code: String, severity: Severity },
}

impl DiagnosticCode for Code {
  fn code(&self) -> &str {
    match self {
      // Lexer
      #[cfg(feature = "lexer")]
      Self::InvalidCharacter => "E001",
      #[cfg(feature = "lexer")]
      Self::InvalidFrontMatter => "E002",
      #[cfg(feature = "lexer")]
      Self::UnterminatedString => "E003",
      #[cfg(feature = "lexer")]
      Self::UnterminatedExpression => "E004",
      #[cfg(feature = "lexer")]
      Self::UnexpectedEof => "E005",
      #[cfg(feature = "lexer")]
      Self::InvalidJsxSelfClosingTag => "E006",
      #[cfg(feature = "lexer")]
      Self::UnterminatedJsxTag => "E007",
      #[cfg(feature = "lexer")]
      Self::InvalidJsxClosingTag => "E008",
      #[cfg(feature = "lexer")]
      Self::InvalidJsxAttribute => "E009",
      #[cfg(feature = "lexer")]
      Self::UnterminatedCodeBlock => "E010",
      #[cfg(feature = "lexer")]
      Self::EmptyFrontMatter => "W001",

      // Parser
      #[cfg(feature = "parser")]
      Self::UnterminatedLink => "P001",
      #[cfg(feature = "parser")]
      Self::UnterminatedImage => "P002",
      #[cfg(feature = "parser")]
      Self::UnterminatedInlineCode => "P003",
      #[cfg(feature = "parser")]
      Self::UnterminatedCodeBlockBlock => "P004",
      #[cfg(feature = "parser")]
      Self::UnterminatedJsxOpenTag => "P005",
      #[cfg(feature = "parser")]
      Self::UnterminatedJsxCloseTag => "P006",
      #[cfg(feature = "parser")]
      Self::UnterminatedJsxExpression => "P007",
      #[cfg(feature = "parser")]
      Self::UnterminatedMdComment => "P008",
      #[cfg(feature = "parser")]
      Self::UnterminatedFrontmatter => "P009",
      #[cfg(feature = "parser")]
      Self::MismatchedJsxCloseTag => "P010",
      #[cfg(feature = "parser")]
      Self::TableShapeMismatch => "P011",
      #[cfg(feature = "parser")]
      Self::StraySetextUnderline => "P012",
      #[cfg(feature = "parser")]
      Self::MissingJsxAttributeValue => "P013",
      #[cfg(feature = "parser")]
      Self::ListMarkerOverflow => "P014",
      #[cfg(feature = "parser")]
      Self::EmptyFrontmatter => "PW001",
      #[cfg(feature = "parser")]
      Self::InvalidFrontmatterYaml => "PW002",
      #[cfg(feature = "parser")]
      Self::HeadingLevelClamped => "PW003",
      #[cfg(feature = "parser")]
      Self::RecoveredUnterminatedJsx => "PW004",

      // Transform
      #[cfg(feature = "transform")]
      Self::ImportFileNotFound => "T001",
      #[cfg(feature = "transform")]
      Self::InvalidLineRange => "T002",
      #[cfg(feature = "transform")]
      Self::RegistryIndexUnreadable => "T003",
      #[cfg(feature = "transform")]
      Self::RegistryIndexMalformed => "T004",
      #[cfg(feature = "transform")]
      Self::RegistryEntryNotFound => "T005",
      #[cfg(feature = "transform")]
      Self::RegistrySourceUnreadable => "T006",
      #[cfg(feature = "transform")]
      Self::ComponentSourceUnreadable => "T007",
      #[cfg(feature = "transform")]
      Self::AssetCopyFailed => "T008",
      #[cfg(feature = "transform")]
      Self::MermaidRenderFailed => "T009",
      #[cfg(feature = "transform")]
      Self::MmdcUnavailable => "TW001",
      #[cfg(feature = "transform")]
      Self::MissingComponentAttr => "TW002",
      #[cfg(feature = "transform")]
      Self::AssetSourceMissing => "TW003",
      #[cfg(feature = "transform")]
      Self::BaseDirNotFound => "TW004",
      #[cfg(feature = "transform")]
      Self::ThemeNotBundled => "TW005",
      #[cfg(feature = "transform")]
      Self::KatexOpts => "TW006",

      #[cfg(feature = "codegen")]
      Self::MalformedJsxTagName => "G001",
      #[cfg(feature = "codegen")]
      Self::MdxTableUnsupported => "GW001",
      #[cfg(feature = "codegen")]
      Self::HtmlExpressionDropped => "GW002",

      // Core
      #[cfg(feature = "core")]
      Self::NoRootDir => "C001",
      #[cfg(feature = "core")]
      Self::NoConfig => "C002",
      #[cfg(feature = "core")]
      Self::NoCollections => "C003",
      #[cfg(feature = "core")]
      Self::CollectionNotFound => "C004",
      #[cfg(feature = "core")]
      Self::CollectionPatternNotFound => "C005",
      #[cfg(feature = "core")]
      Self::CollectionSchemaNotFound => "C006",
      #[cfg(feature = "core")]
      Self::InvalidConfig => "C007",
      #[cfg(feature = "core")]
      Self::InvalidConfigPath => "C008",
      #[cfg(feature = "core")]
      Self::ConfigExists => "CW001",

      // Shared
      Self::IoRead => "S001",
      Self::IoWrite => "S002",
      Self::IoCreateDir => "S003",
      Self::JsonDeserialize => "S004",
      Self::JsonSerialize => "S005",
      Self::LockPoisoned => "S006",
      Self::IoRecoverable => "SW001",

      Self::Custom { code, .. } => code.as_str(),
    }
  }

  fn severity(&self) -> Severity {
    match self {
      // Lexer errors
      #[cfg(feature = "lexer")]
      Self::InvalidCharacter
      | Self::InvalidFrontMatter
      | Self::UnterminatedString
      | Self::UnterminatedExpression
      | Self::UnexpectedEof
      | Self::InvalidJsxSelfClosingTag
      | Self::UnterminatedJsxTag
      | Self::InvalidJsxClosingTag
      | Self::InvalidJsxAttribute
      | Self::UnterminatedCodeBlock => Severity::Error,
      #[cfg(feature = "lexer")]
      Self::EmptyFrontMatter => Severity::Warning,

      // Parser errors
      #[cfg(feature = "parser")]
      Self::UnterminatedLink
      | Self::UnterminatedImage
      | Self::UnterminatedInlineCode
      | Self::UnterminatedCodeBlockBlock
      | Self::UnterminatedJsxOpenTag
      | Self::UnterminatedJsxCloseTag
      | Self::UnterminatedJsxExpression
      | Self::UnterminatedMdComment
      | Self::UnterminatedFrontmatter
      | Self::MismatchedJsxCloseTag
      | Self::TableShapeMismatch
      | Self::StraySetextUnderline
      | Self::MissingJsxAttributeValue
      | Self::ListMarkerOverflow => Severity::Error,
      #[cfg(feature = "parser")]
      Self::EmptyFrontmatter
      | Self::InvalidFrontmatterYaml
      | Self::HeadingLevelClamped
      | Self::RecoveredUnterminatedJsx => Severity::Warning,

      // Transform errors
      #[cfg(feature = "transform")]
      Self::ImportFileNotFound
      | Self::InvalidLineRange
      | Self::RegistryIndexUnreadable
      | Self::RegistryIndexMalformed
      | Self::RegistryEntryNotFound
      | Self::RegistrySourceUnreadable
      | Self::ComponentSourceUnreadable
      | Self::AssetCopyFailed
      | Self::MermaidRenderFailed => Severity::Error,
      #[cfg(feature = "transform")]
      Self::MmdcUnavailable
      | Self::MissingComponentAttr
      | Self::AssetSourceMissing
      | Self::BaseDirNotFound
      | Self::ThemeNotBundled
      | Self::KatexOpts => Severity::Warning,

      #[cfg(feature = "codegen")]
      Self::MalformedJsxTagName => Severity::Error,
      #[cfg(feature = "codegen")]
      Self::MdxTableUnsupported | Self::HtmlExpressionDropped => Severity::Warning,

      // Core errors / warnings
      #[cfg(feature = "core")]
      Self::NoRootDir
      | Self::NoConfig
      | Self::NoCollections
      | Self::CollectionNotFound
      | Self::CollectionPatternNotFound
      | Self::CollectionSchemaNotFound
      | Self::InvalidConfig
      | Self::InvalidConfigPath => Severity::Error,
      #[cfg(feature = "core")]
      Self::ConfigExists => Severity::Warning,

      // Shared
      Self::IoRead
      | Self::IoWrite
      | Self::IoCreateDir
      | Self::JsonDeserialize
      | Self::JsonSerialize
      | Self::LockPoisoned => Severity::Error,
      Self::IoRecoverable => Severity::Warning,

      Self::Custom { severity, .. } => *severity,
    }
  }
}
