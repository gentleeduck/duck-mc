# dmc-highlight internals

## Lazy bundle

```rust
pub fn get() -> &'static SyntaxBundle {
    static B: OnceLock<SyntaxBundle> = OnceLock::new();
    B.get_or_init(|| {
        let mut builder = SyntaxSet::load_from_folder(/* ... */)
            .expect("load grammars-sublime")
            .into_builder();
        builder.add_plain_text_syntax();
        let syntaxes = builder.build();
        let themes = ThemeSet::load_from_folder(/* ... */)
            .expect("load themes-bat");
        SyntaxBundle { syntaxes, themes }
    })
}
```

One parse per process. ~25-100 ms cost on first call (themes ~5-15
ms each + grammars ~50-200 ms total). Free thereafter.

## Plain-text fallback

`SyntaxSet::load_from_folder` returns a set with no plain-text
grammar. `find_syntax_plain_text` would panic. The builder dance
adds it explicitly so unknown-language code blocks render plainly:

```rust
builder.add_plain_text_syntax();
```

Without this, a code fence with an unknown lang string (`bash` typo,
truly niche language) would crash. Now it renders as `<pre><code>`
with no token spans.

## Theme + grammar lookup

```rust
let syntax = lang.and_then(|l| {
    bundle.syntaxes.find_syntax_by_extension(l)
        .or_else(|| bundle.syntaxes.find_syntax_by_token(l))
        .or_else(|| bundle.syntaxes.find_syntax_by_name(l))
})
.unwrap_or_else(|| bundle.syntaxes.find_syntax_plain_text());
```

Three-stage lookup:

1. `find_syntax_by_extension(s)` -> matches the grammar's
   `file_extensions:` list (e.g. `"tsx"`).
2. `find_syntax_by_token(s)` -> matches the same list (used for
   short tokens).
3. `find_syntax_by_name(s)` -> matches the `name:` header inside
   the `.sublime-syntax` file (e.g. `"TypeScriptReact"`).

Theme lookup is single-stage:

```rust
let theme = bundle.themes.themes
    .get(theme_name)
    .or_else(|| bundle.themes.themes.values().next())
    .expect("at least one theme bundled");
```

Falls back to the first bundled theme on miss; never panics.

## `highlight_code_multi` pipeline

```rust
let mut parse_state = ParseState::new(syntax);
let mut highlight_states: Vec<HighlightState> = highlighters
    .iter()
    .map(|h| HighlightState::new(h, ScopeStack::new()))
    .collect();

for line in LinesWithEndings::from(code) {
    let ops = parse_state.parse_line(line, &bundle.syntaxes).unwrap_or_default();

    let mut per_theme: Vec<Vec<(Style, &str)>> = Vec::with_capacity(theme_names.len());
    for (i, st) in highlight_states.iter_mut().enumerate() {
        let toks: Vec<(Style, &str)> = RangedHighlightIterator::new(st, &ops, line, &highlighters[i])
            .map(|(style, text, _)| (style, text))
            .collect();
        per_theme.push(toks);
    }

    let token_count = per_theme.iter().map(Vec::len).min().unwrap_or(0);
    // zip across themes; merge adjacent same-style tokens
}
```

`parse_line` runs once per line. `RangedHighlightIterator` walks the
same op list per theme to produce `(Style, text, range)` tuples.
Boundaries are theme-independent; only colors differ.

## Adjacent merge

```rust
fn styles_match(a: &[Style], b: &[Style]) -> bool {
    a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| {
        x.foreground == y.foreground
            && x.background == y.background
            && x.font_style == y.font_style
    })
}

fn join_adjacent<'a>(a: &'a str, b: &'a str) -> Option<&'a str> {
    let a_end = a.as_ptr() as usize + a.len();
    let b_start = b.as_ptr() as usize;
    if a_end != b_start { return None; }
    let bytes = unsafe { std::slice::from_raw_parts(a.as_ptr(), a.len() + b.len()) };
    std::str::from_utf8(bytes).ok()
}
```

`styles_match` compares all themes' styles for two consecutive
tokens. `join_adjacent` extends the first token's slice to cover the
second when they border in the same source string. Merged tokens
reduce span count, matches shiki coalescing.

The unsafe `from_raw_parts` is safe here because both slices come
from the same `&str` (the input `code` argument).

## Asset folders

```
dmc-highlight/
|- assets/
|   |- themes-bat/      .tmTheme files
|   `- grammars-sublime/  .sublime-syntax files
|- build.rs             scans both folders, generates Theme/Grammar enums
|- src/lib.rs           include!(OUT_DIR/assets_gen.rs)
`- scripts/
    `- convert-shiki-assets.mjs    refresh helper
```

## Memory

`SyntaxBundle` heap allocation totals ~5-15 MB depending on bundle.
Process pays once via `OnceLock`. Drop is at process exit.

## Thread safety

`SyntaxSet` and `ThemeSet` are immutable after construction (just
read). `SyntaxBundle::get()` returns `&'static`, safe to share
across threads. Used from rayon `par_iter` in
`Collection::process` without locks.

`HighlightState` and `ParseState` are mutable per-call; never shared.
`highlight_code_multi` constructs fresh ones per invocation.
