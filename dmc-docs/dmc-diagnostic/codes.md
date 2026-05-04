# Code reference

Exhaustive list of every `Code` variant, sorted by code-string.

## Lexer

| code | variant | severity | trigger |
|------|---------|----------|---------|
| E001 | `InvalidCharacter` | error | byte outside expected lex set |
| E002 | `InvalidFrontMatter` | error | malformed `---...---` block header |
| E003 | `UnterminatedString` | error | string literal in JSX attr missing `"` |
| E004 | `UnterminatedExpression` | error | JSX `{...}` without closer |
| E005 | `UnexpectedEof` | error | EOF mid-token |
| E006 | `InvalidJsxSelfClosingTag` | error | malformed `<X />` |
| E007 | `UnterminatedJsxTag` | error | `<X` without `>` |
| E008 | `InvalidJsxClosingTag` | error | malformed `</X>` |
| E009 | `InvalidJsxAttribute` | error | malformed JSX attr |
| E010 | `UnterminatedCodeBlock` | error | code fence without close |
| W001 | `EmptyFrontMatter` | warning | empty `---\n---` block |

## Parser

| code | variant | severity | trigger |
|------|---------|----------|---------|
| P001 | `UnterminatedLink` | error | `[text](href` no close |
| P002 | `UnterminatedImage` | error | `![alt](src` no close |
| P003 | `UnterminatedInlineCode` | error | backtick without partner |
| P004 | `UnterminatedCodeBlockBlock` | error | parser-side fence mismatch |
| P005 | `UnterminatedJsxOpenTag` | error | parser saw `<X` no close |
| P006 | `UnterminatedJsxCloseTag` | error | `</X` no close |
| P007 | `UnterminatedJsxExpression` | error | `{` no `}` |
| P008 | `UnterminatedMdComment` | error | `{/* ... */}` mismatched |
| P009 | `UnterminatedFrontmatter` | error | YAML frontmatter mid-doc |
| P010 | `MismatchedJsxCloseTag` | error | `<a></b>` |
| P011 | `TableShapeMismatch` | error | row column count mismatch |
| P012 | `StraySetextUnderline` | error | `===` outside heading context |
| P013 | `MissingJsxAttributeValue` | error | `name=` no value |
| P014 | `ListMarkerOverflow` | error | numeric list marker too large |
| PW001 | `EmptyFrontmatter` | warning | empty YAML block |
| PW002 | `InvalidFrontmatterYaml` | warning | YAML parse error |
| PW003 | `HeadingLevelClamped` | warning | `#######` clamped to h6 |
| PW004 | `RecoveredUnterminatedJsx` | warning | parser synthesised self-close for unterminated tag |

## Transform

| code | variant | severity | source |
|------|---------|----------|--------|
| T001 | `ImportFileNotFound` | error | `code-import` `file=` unreadable |
| T002 | `InvalidLineRange` | error | `code-import` `{ranges}` malformed |
| T003 | `RegistryIndexUnreadable` | error | `component-preview` index unreadable |
| T004 | `RegistryIndexMalformed` | error | `component-preview` index not JSON |
| T005 | `RegistryEntryNotFound` | error | `component-preview` `name=` missing |
| T006 | `RegistrySourceUnreadable` | error | `component-preview` first file unreadable |
| T007 | `ComponentSourceUnreadable` | error | `component-source` `path=` unreadable |
| T008 | `AssetCopyFailed` | error | `copy-linked-files` write failed |
| T009 | `MermaidRenderFailed` | error | `mmdc` exit non-zero |
| TW001 | `MmdcUnavailable` | warning | `mmdc` not on PATH (mermaid no-ops) |
| TW002 | `MissingComponentAttr` | warning | `component-preview` / `component-source` missing required attr |
| TW003 | `AssetSourceMissing` | warning | `copy-linked-files` referenced asset missing |
| TW004 | `BaseDirNotFound` | warning | non-disk source w/o explicit `base_dir` |

## Codegen

| code | variant | severity | source |
|------|---------|----------|--------|
| G001 | `MalformedJsxTagName` | error | empty / invalid tag name on JSX node |
| GW001 | `MdxTableUnsupported` | warning | `MdxBodyEmitter` dropped a Table node |
| GW002 | `HtmlExpressionDropped` | warning | `HtmlEmitter` dropped a `JsxExpression` |

## Custom

```rust
Custom { code: String, severity: Severity }
```

Escape hatch for third-party transformer authors. Never use for built-in
codes; add a typed variant upstream instead.
