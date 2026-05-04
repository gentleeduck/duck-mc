# Grammars

Grammar bundle lives in `assets/grammars-sublime/`. Each
`.sublime-syntax` file becomes one variant of the generated `Grammar`
enum.

## Bundled list

Run-time enumeration:

```rust
use dmc_highlight::GRAMMARS;

for g in GRAMMARS {
    println!("{}", g.name());
}
```

The bundle is the shiki/VS Code grammar set converted to Sublime
syntax format via the `scripts/convert-shiki-assets.mjs` helper. Run
the script to refresh. Covers ~250 languages (rust, typescript, tsx,
jsx, python, go, ruby, bash, json, yaml, toml, sql, html, css, scss,
markdown, mdx, dockerfile, etc).

## Lookup chain

`highlight_code(code, lang, theme)` resolves `lang` by:

1. `find_syntax_by_extension(lang)` (e.g. `"rs"`, `"tsx"`).
2. `find_syntax_by_token(lang)` (matches `.sublime-syntax`'s
   `file_extensions` field).
3. `find_syntax_by_name(lang)` (matches the `name:` field, e.g.
   `"TypeScriptReact"`).
4. Fallback: `find_syntax_plain_text()`.

Step 4 only works because `SyntaxBundle::get` calls
`SyntaxSetBuilder::add_plain_text_syntax` after loading the folder.
Without that, `find_syntax_plain_text()` panics.

## Naming

`Grammar::name()` returns the file stem (e.g. `"tsx"`,
`"TypsecriptReact"`). The variant identifier is sanitised PascalCase.

Note: file stems sometimes diverge from the grammar's `name:` header.
For instance, `TypsecriptReact.sublime-syntax` (file stem with typo)
has `name: TypeScriptReact` inside. Lookup chain handles both.

## Adding a grammar

1. Drop `MyLang.sublime-syntax` into `dmc-highlight/assets/grammars-sublime/`.
2. Rebuild. `build.rs` re-scans on `cargo:rerun-if-changed=assets/grammars-sublime`.
3. New variant `Grammar::MyLang` is generated.

## Plain-text fallback

Code blocks with unknown lang strings render as plain text rather than
erroring. `pretty-code` and the standalone free functions both rely on
this so a niche language never breaks the build.
