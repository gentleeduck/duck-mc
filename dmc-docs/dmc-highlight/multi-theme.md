# Multi-theme highlighting

`highlight_code_multi` does one parse + scope walk and resolves color
for every theme in lock-step. Cuts per-file syntect cost ~25% vs
calling `highlight_code` once per theme.

## Why it works

Token boundaries come from grammar scope changes. Themes only set
colors for those scopes. So:

1. `ParseState::parse_line(line, syntax_set)` runs once per line.
2. Output is a `Vec<(usize, ScopeStackOp)>` (boundary positions +
   stack ops). Theme-independent.
3. For each theme, walk the same op list with that theme's
   `HighlightState`. Every theme yields the same number of tokens at
   the same boundaries.
4. Zip across themes: each output token gets one slice + N styles.

## API recap

```rust
pub fn highlight_code_multi<'a>(
    code: &'a str,
    lang: Option<&str>,
    theme_names: &[&str],
) -> Vec<Vec<MultiToken<'a>>>
```

```rust
pub struct MultiToken<'a> {
    pub text: &'a str,
    pub styles: Vec<Style>,  // one per theme, in input order
}
```

## Adjacent merge

Two adjacent tokens whose styles match across every theme are merged
into one. Halves the span count gap vs shiki, which coalesces the
same way.

```rust
fn styles_match(a: &[Style], b: &[Style]) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b.iter())
            .all(|(x, y)| {
                x.foreground == y.foreground
                    && x.background == y.background
                    && x.font_style == y.font_style
            })
}
```

The merge step concatenates two `&str` slices only when they border
the same source string (pointer arithmetic check via `join_adjacent`).
Safe because both slices come from the same input borrow.

## CSS variable emission

`pretty-code` reads `MultiToken.styles[i]` and emits per-mode CSS:

- primary mode (e.g. dark): unprefixed `color:#xxxxxx`
- other modes: `--dmc-{mode}:#xxxxxx` per token, plus
  `--dmc-{mode}-bg:#xxxxxx` on the wrapping `<pre>`

Consumer CSS swaps modes by rebinding `color` /
`background-color` to the active variable inside whichever
selector controls the theme.

## Cost vs single-theme

| fixture | single-theme | multi-theme (2 themes) |
|---------|-------------|----------------------|
| short ~80 B | 3.0 us | 3.0 us |
| medium ~1 KB | 305 us | 484 us |
| heavy ~2 KB | 895 us | 1469 us |

Multi-theme adds ~60% per file, not 100%, because the parse step (the
bulk of work) runs once.

## When to single-theme

Set `PrettyCodeOptions::theme = PrettyCodeTheme::Single("...")`
to skip multi-mode output entirely. Token style is one `color:#xxx`
per token, no CSS vars. Recommended when the docs ship one theme.
