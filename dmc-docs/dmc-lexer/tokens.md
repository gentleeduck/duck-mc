# TokenKind reference

Full enumeration of `dmc_lexer::token::TokenKind`. All variants are `pub`. The "trigger" column shows the source bytes that produce the token.

## Table

| Variant | Trigger | Notes |
|---|---|---|
| `FrontmatterStart` | `---\n` at byte offset 0 | Only when followed by a closing `\n---` later |
| `FrontmatterContent` | YAML body between `---` delimiters | Raw, unparsed |
| `FrontmatterEnd` | closing `---` at column 0 | Sets `frontmatter_reserved = true` |
| `ThematicBreak` | `---` not at file start, or `***` / `___` on EOL | Falls out when frontmatter rules don't apply |
| `Import` | `import` at column 0 | Whole statement, multi-line `{}` aware |
| `Export` | `export` at column 0 | Whole statement, multi-line `{}` aware |
| `Heading(u8)` | `#` x 1-6 followed by space | `u8` carries the level |
| `Text` | catch-all | Plain content between markers |
| `Bold(u8)` | `**` or `__` (or `***` not on EOL) | `u8` is the delimiter count (2 or 3) |
| `Italic(u8)` | `*` or `_` | `u8` is the delimiter count (1) |
| `Strike(u8)` | `~~` | `u8` is the delimiter count (2) |
| `JsxOpenTagStart` | `<` followed by alpha/`/`/`>` | `<` |
| `JsxOpenTagEnd` | `>` ending an open tag | `>` |
| `JsxCloseTagStart` | `</` | `</` |
| `JsxCloseTagEnd` | `>` ending a close tag | `>` |
| `JsxSelfClosingEnd` | `/>` | self-closing terminator |
| `JsxTagName` | run of alphanumeric + `.` after `<` or `</` | e.g. `Button`, `Foo.Bar` |
| `JsxAttributeName` | run of alphanumeric + `-` inside a tag | `Display` impl prints "JsxAttribute" |
| `JsxAttributeValue` | reserved variant; emitted as `String` or via `ExpressionStart`/`ExpressionEnd` instead | not emitted directly by current lexers |
| `ExpressionStart` | `{` | top-level or attribute expression |
| `ExpressionEnd` | `}` | balanced with depth counter |
| `BlockQuote` | `>` not inside a JSX tag | bare `>` at any column |
| `OrderedListItem` | digits followed by `.` or `)` | the digit run is the lexeme |
| `UnorderedListItem` | `-` followed by space | the dash run is the lexeme |
| `CodeStart(u8)` | `` ` `` x N | `u8` is N. >= 3 = fenced; 1-2 = inline |
| `CodeEnd(u8)` | matching closing backticks | `u8` matches the open count |
| `Bracket` | `[` and matching `]` | emitted twice per link (open + close) |
| `Bang` | `!` immediately before `[` | image marker |
| `ParenOpen` | `(` | bare or in `[text](href)` |
| `ParenClose` | `)` | bare or in `[text](href)` |
| `Eq` | `=` | JSX attribute assignment |
| `String` | content between matched `"..."` or `'...'` in JSX | the inner text only |
| `HTMLCommentStart` | `<!--` | |
| `HTMLCommentEnd` | `-->` | |
| `Autolink` | `<https://...>` or `<a@b.c>` | gated by `is_angle_autolink` heuristic |
| `MarkdownCommentStart` | `{/*` | |
| `MarkdownCommentEnd` | `*/}` | |
| `HardBreak` | >= 2 consecutive `\n` | blank-line block separator |
| `SoftBreak` | single `\n` between content | inline line break |
| `Newline` | `\n` (trivia) | dropped by `emit` |
| `Whitespace` | run of ` `, `\t`, `\r` | preserved (see `internals.md`) |
| `Quote` | `"` or `'` (trivia) | dropped by `emit`; only fires inside JSX attributes |
| `Eof` | end of source | emitted once by `scan_tokens` |

## Variant-by-variant detail

### Frontmatter

`FrontmatterStart`, `FrontmatterContent`, `FrontmatterEnd` form a triple emitted by `lex_frontmatter` when:

1. The `---` run is exactly 3 dashes.
2. `frontmatter_reserved` is `false`.
3. `current` is at file start (offset <= 3 bytes).
4. A `\n---` exists later in the source.

If any condition fails, the dashes emit a single `ThematicBreak` instead.

### Block-level

- `Heading(u8)` - `#` x 1-6 followed by ` `. Without a trailing space, falls back to `lex_text`.
- `ThematicBreak` - `---` not at file start, or `***` / `___` ending the line.
- `BlockQuote` - bare `>` (not part of a JSX tag).
- `OrderedListItem` - digit run before `.` or `)`. Stops at the digits; the punctuation is consumed by the next call.
- `UnorderedListItem` - `-` followed by space. The dash itself is the lexeme.

### Inline marks

- `Bold(2)` - `**` or `__`.
- `Bold(3)` - `***` mid-line (not on EOL).
- `Italic(1)` - single `*` or `_`.
- `Strike(2)` - `~~`.
- `Text` - anything else, including escaped `\*` / `\_` / `\<` etc.

### JSX

- `JsxOpenTagStart` - `<` before alpha or `>`.
- `JsxCloseTagStart` - `</`.
- `JsxTagName` - run of `[A-Za-z0-9.]`.
- `JsxAttributeName` - run of `[A-Za-z0-9-]`.
- `Eq` - `=` inside a JSX tag.
- `String` - content between matching `"..."` or `'...'`. The quotes themselves are emitted as `Quote` (then dropped by `emit`).
- `ExpressionStart` / `Text` / `ExpressionEnd` - `{ ... }` attribute or top-level expression.
- `JsxSelfClosingEnd` - `/>`.
- `JsxOpenTagEnd` / `JsxCloseTagEnd` - `>` closing the respective opener.

### Code

- `CodeStart(N)` - `` ` `` x N at the run's beginning.
- `CodeEnd(N)` - closing run of length matching the open. For fenced (N >= 3) the close must sit at column 0.
- `Text` - info-string after a fence (e.g. `js`, `rust title="x.rs"`) and the body.

### Comments

- `HTMLCommentStart` (`<!--`) + `Text` body + `HTMLCommentEnd` (`-->`).
- `MarkdownCommentStart` (`{/*`) + `Text` body + `MarkdownCommentEnd` (`*/}`).

### Autolink

`<URL>` or `<email>`. `is_angle_autolink` peeks for a `>` before any space/newline and validates either `://` (URL) or RFC-shaped local@domain (email).

### Statements

`Import` / `Export` cover the entire statement up to the terminating newline at brace depth 0. `import { a, b } from "x";` lexes to one `Import` token even with line-broken braces.

### Breaks

- `HardBreak` - 2+ consecutive `\n`.
- `SoftBreak` - 1 `\n`.

### Trivia

`is_trivia()` returns `true` for `Whitespace`, `Newline`, `Quote`. Of those, only `Newline` and `Quote` are dropped by `emit`. `Whitespace` is preserved. See `internals.md`.

### Eof

Pushed once after the main loop exits.
