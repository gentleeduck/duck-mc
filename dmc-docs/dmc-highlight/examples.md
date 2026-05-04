# dmc-highlight examples

## Single-theme highlight

```rust
use dmc_highlight::highlight_code;

let lines = highlight_code(
    "fn main() { println!(\"hi\"); }",
    Some("rust"),
    "Catppuccin Mocha",
);

for line in lines {
    for (style, text) in line {
        let fg = style.foreground;
        print!("\x1b[38;2;{};{};{}m{}", fg.r, fg.g, fg.b, text);
    }
}
```

Returns `Vec<line>` where each line is `Vec<(Style, &str)>`. Falls
back to plain-text grammar when `lang` is unknown; falls back to first
bundled theme when theme name is unknown.

## Multi-theme highlight

```rust
use dmc_highlight::{highlight_code_multi, MultiToken};

let themes = ["Catppuccin Latte", "Catppuccin Mocha"];
let lines = highlight_code_multi(
    "let x = 42;",
    Some("rust"),
    &themes,
);

for line in lines {
    for MultiToken { text, styles } in line {
        let light = styles[0].foreground;
        let dark  = styles[1].foreground;
        println!(
            "{} -> light=#{:02x}{:02x}{:02x} dark=#{:02x}{:02x}{:02x}",
            text,
            light.r, light.g, light.b,
            dark.r, dark.g, dark.b,
        );
    }
}
```

One parse + scope walk; each theme contributes only color resolution.
Token boundaries match across themes (driven by grammar, not theme).

## Enum-typed lookup

```rust
use dmc_highlight::{SyntaxBundle, Theme, Grammar};

let bundle = SyntaxBundle::get();
let lines = bundle.highlight(
    "console.log('hi');",
    Grammar::Typescript,
    Theme::Nord,
);

let _ = lines;
```

Compile-time guaranteed: enum variants exist only when the grammar /
theme is bundled. Use `Theme::from_name(s)` / `Grammar::from_name(s)`
for user-supplied strings.

## Theme + grammar enumeration

```rust
use dmc_highlight::{THEMES, GRAMMARS};

for t in THEMES {
    println!("theme: {}", t.name());
}
for g in GRAMMARS {
    println!("grammar: {}", g.name());
}
```

Use to populate UI pickers or CLI `--list-themes` output.
