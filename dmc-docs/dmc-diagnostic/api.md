# dmc-diagnostic API

## `Code` enum

```rust
pub enum Code {
    // lexer (feature = "lexer")
    InvalidCharacter,
    InvalidFrontMatter,
    UnterminatedString,
    UnterminatedExpression,
    UnexpectedEof,
    InvalidJsxSelfClosingTag,
    UnterminatedJsxTag,
    InvalidJsxClosingTag,
    InvalidJsxAttribute,
    UnterminatedCodeBlock,
    EmptyFrontMatter,

    // parser (feature = "parser")
    UnterminatedLink,
    UnterminatedImage,
    UnterminatedInlineCode,
    UnterminatedCodeBlockBlock,
    UnterminatedJsxOpenTag,
    UnterminatedJsxCloseTag,
    UnterminatedJsxExpression,
    UnterminatedMdComment,
    UnterminatedFrontmatter,
    MismatchedJsxCloseTag,
    TableShapeMismatch,
    StraySetextUnderline,
    MissingJsxAttributeValue,
    ListMarkerOverflow,
    EmptyFrontmatter,
    InvalidFrontmatterYaml,
    HeadingLevelClamped,
    RecoveredUnterminatedJsx,

    // transform (feature = "transform")
    ImportFileNotFound,
    InvalidLineRange,
    RegistryIndexUnreadable,
    RegistryIndexMalformed,
    RegistryEntryNotFound,
    RegistrySourceUnreadable,
    ComponentSourceUnreadable,
    AssetCopyFailed,
    MermaidRenderFailed,
    MmdcUnavailable,
    MissingComponentAttr,
    AssetSourceMissing,
    BaseDirNotFound,

    // codegen (feature = "codegen")
    MalformedJsxTagName,
    MdxTableUnsupported,
    HtmlExpressionDropped,

    // always
    Custom { code: String, severity: Severity },
}
```

Path: `dmc_diagnostic::Code`.

## `DiagnosticCode for Code`

```rust
impl DiagnosticCode for Code {
    fn code(&self) -> &str;       // canonical id ("E001", "T009", etc)
    fn severity(&self) -> Severity; // Error | Warning
}
```

## `metadata` module

```rust
pub use metadata::{Origin, SourceMeta};

pub struct SourceMeta {
    pub path: Arc<str>,
    pub version: u64,
    pub origin: Origin,
}

pub enum Origin {
    File(PathBuf),
    Stdin,
    Inline(&'static str),
    Memory,
}
```

Path: `dmc_diagnostic::metadata::{SourceMeta, Origin}`. Carries source
location info across layer boundaries; every diagnostic span ties back
to a `SourceMeta`.

## Re-exports from `duck-diagnostic`

`Code` plugs into `duck_diagnostic::DiagnosticEngine<C>`,
`Diagnostic<C>`, `Label`, `Severity`, `Span`. Consumer crates use:

```rust
use duck_diagnostic::{Diagnostic, DiagnosticEngine, Label, Span};
use dmc_diagnostic::Code;

let mut engine: DiagnosticEngine<Code> = DiagnosticEngine::new();
engine.emit(Diagnostic::new(Code::InvalidCharacter, "bad byte"));
```
