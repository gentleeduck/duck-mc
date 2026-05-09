# `bare-url`

Wraps bare `https://…` / `http://…` URLs in `<a href="…">` so they
render as clickable links without explicit Markdown link syntax.

- **Source:** `dmc-transform/src/builtin/bare_url.rs`
- **Feature flag:** none
- **Config:** none

## Behavior

Applies inside text nodes only. Skips:

- URLs already inside a Markdown `[text](url)` link.
- URLs inside fenced or inline code spans.
- Trailing punctuation (`.`, `,`, `)`, `]`, `}`) is excluded from the
  link target.

Mirrors GFM autolink-literal semantics for plain http/https URLs.

## Sidecar opt-out

None — the matching JS plugin (`remark-gfm`'s `autolink: literal`) is
absorbed by `disable-gfm`'s gate. Drop GFM globally via
`markdown.gfm: false`.
