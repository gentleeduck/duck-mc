# dmc-highlight API

Every public symbol. Canonical paths.

## SyntaxBundle

```rust
pub struct SyntaxBundle {
    pub syntaxes: SyntaxSet,
    pub themes: ThemeSet,
}
```

Path: `dmc_highlight::SyntaxBundle`.

### `SyntaxBundle::get`

```rust
pub fn get() -> &'static SyntaxBundle
```

Process-global bundle. ~25-100 ms one-time parse on first call (themes
+ grammars), free thereafter. Plain-text grammar added via
`SyntaxSetBuilder::add_plain_text_syntax` so unknown-language fallback
works.

### `SyntaxBundle::highlight`

```rust
pub fn highlight<'a>(
    &'a self,
    code: &'a str,
    lang: Grammar,
    theme: Theme,
) -> Vec<Vec<(Style, &'a str)>>
```

Highlight `code` with the enum-typed grammar + theme. Returns
`Vec<line>` where each line is `Vec<(Style, slice)>`.

### `SyntaxBundle::highlight_by_name`

```rust
pub fn highlight_by_name<'a>(
    &'a self,
    code: &'a str,
    lang: &str,
    theme: Theme,
) -> Vec<Vec<(Style, &'a str)>>
```

Same as `highlight` but takes a free-form lang name (matched by
extension, then token, then full name; falls back to plain text).

## Free functions

### `highlight_code`

```rust
pub fn highlight_code<'a>(
    code: &'a str,
    lang: Option<&str>,
    theme_name: &str,
) -> Vec<Vec<(Style, &'a str)>>
```

Path: `dmc_highlight::highlight_code`. Forgiving: unknown `lang` falls
back to plain-text grammar, unknown `theme_name` falls back to first
bundled theme. Used by `pretty-code` single-theme path.

### `highlight_code_multi`

```rust
pub fn highlight_code_multi<'a>(
    code: &'a str,
    lang: Option<&str>,
    theme_names: &[&str],
) -> Vec<Vec<MultiToken<'a>>>
```

Path: `dmc_highlight::highlight_code_multi`. Single-tokenize, multi-color.
One parse + scope walk. Each theme contributes only color resolution.
Token boundaries are theme-independent (driven by grammar scope ops),
so all themes contribute styles for the same source slices. Adjacent
tokens whose styles match across every theme are merged (matches
shiki coalescing).

## Types

### `MultiToken`

```rust
pub struct MultiToken<'a> {
    pub text: &'a str,
    pub styles: Vec<Style>,
}
```

Path: `dmc_highlight::MultiToken`. One highlighted token: source slice
plus per-theme styles in same order as `theme_names` slice.

### `Theme` (build-time generated)

```rust
pub enum Theme { CatppuccinMocha, Nord, ... }
impl Theme {
    pub const fn name(self) -> &'static str;
    pub fn from_name(s: &str) -> Option<Self>;
}
pub const THEMES: &[Theme];
```

Path: `dmc_highlight::Theme`. One variant per file in
`assets/themes-bat/`. `name()` returns canonical syntect lookup key
(file stem). `from_name()` is the inverse.

### `Grammar` (build-time generated)

```rust
pub enum Grammar { Rust, Typescript, TypsecriptReact, ... }
impl Grammar {
    pub const fn name(self) -> &'static str;
    pub fn from_name(s: &str) -> Option<Self>;
}
pub const GRAMMARS: &[Grammar];
```

Path: `dmc_highlight::Grammar`. Same shape as `Theme` but for grammars.

## Re-exports

```rust
pub use syntect::highlighting::{Color, Style as HlStyle};
```

Consumers (e.g. `dmc-transform::pretty-code`) take `HlStyle` and
`Color` without depending on `syntect` themselves.
