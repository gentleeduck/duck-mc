# Themes

Theme bundle lives in `assets/themes-bat/`. Each `.tmTheme` file
becomes one variant of the generated `Theme` enum (via `build.rs`).

## Bundled list

Run-time enumeration:

```rust
use dmc_highlight::THEMES;

for t in THEMES {
    println!("{}", t.name());
}
```

Static list (current bundle):

- `1337`
- `ansi`
- `base16-256`
- `base16`
- `Catppuccin Frappe`
- `Catppuccin Latte`
- `Catppuccin Macchiato`
- `Catppuccin Mocha`
- `Coldark-Cold`
- `Coldark-Dark`
- `DarkNeon`
- `gruvbox-dark`
- `gruvbox-light`
- `Nord`
- `OneHalfDark`
- `OneHalfLight`
- `Solarized (dark)`
- `Solarized (light)`
- `tokyo-night`
- `TwoDark`

## Naming

`Theme::name()` returns the file stem verbatim. The variant identifier
is sanitised PascalCase (so `Catppuccin Mocha` becomes
`Theme::CatppuccinMocha`). Look up by string via
`Theme::from_name("Catppuccin Mocha")`.

## Resolution rules

Highlight free-functions resolve theme name by:

1. `bundle.themes.themes.get(name)` exact lookup.
2. If miss, fall back to first bundled theme via
   `bundle.themes.themes.values().next()`.

So unknown theme names never panic. Use `Theme::from_name(s).is_some()`
to check up front.

## Adding a theme

1. Drop `MyTheme.tmTheme` into `dmc-highlight/assets/themes-bat/`.
2. Rebuild. `build.rs` re-scans on `cargo:rerun-if-changed=assets/themes-bat`.
3. New variant `Theme::MyTheme` is generated.

## Default in PrettyCode

`dmc_transform::PrettyCode::default()` ships a Multi:
- `light: Catppuccin Latte`
- `dark: Catppuccin Mocha`

with `default_mode: "dark"`. Override via `PrettyCodeOptions`.
