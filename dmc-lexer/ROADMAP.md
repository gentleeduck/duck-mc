# dmc-lexer roadmap

## Current status

- The lexer feeds the parser/codegen path that now passes
  CommonMark `652/652` and GFM `670/670` end to end.
- Tokenization covers markdown structure, GFM tables/task lists/autolinks,
  HTML blocks, JSX, MDX expressions/comments, ESM, and frontmatter.

## Behavior notes

### Column-aware whitespace

- The scanner tracks logical columns separately from byte offsets.
- `Whitespace(n)` stores the raw byte width, but block decisions use
  visual column deltas so mixed spaces/tabs follow CommonMark rules.

### Tab-stop handling

- `advance` and `advance_bytes` both snap `\t` to the next 4-column stop.
- Indented-code detection uses `column - start_column`, so `1-3`
  spaces plus a tab correctly count as a 4-column indent.

### ESM detection

- `import` and `export` are only treated as MDX ESM when they start at
  column 0 and are followed by a space or tab.
- The body is consumed opaquely with brace/string/template/comment
  tracking until the next top-level newline.

### Partial-tab consumption in blockquote and list markers

- After `>` and after list markers, a following tab is only consumed
  when it advances exactly one visual column (`column % 4 == 3`).
- Otherwise the tab is left in the stream so the parser can still see
  the real continuation or indented-code indent instead of losing up
  to three columns inside the marker token.

## Remaining work

- Add more lexer-only golden tests if token-stream snapshots become
  useful.
- Keep fuzz/invariant coverage running as MDX and HTML edge cases grow.
