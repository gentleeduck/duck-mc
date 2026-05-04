# build.rs (asset codegen)

`dmc-highlight/build.rs` scans the asset folders at compile time and
emits the `Theme` and `Grammar` enums plus the `THEMES` / `GRAMMARS`
slices.

## Scan

```rust
let themes   = scan(&crate_root.join("assets/themes-bat"),       "tmTheme");
let grammars = scan(&crate_root.join("assets/grammars-sublime"), "sublime-syntax");
```

For each entry in the directory:

1. Skip if extension does not match (`tmTheme` / `sublime-syntax`).
2. Take the file stem.
3. Push to a `Vec<String>`.
4. Sort + dedup.

Output is a list of file-stem strings.

## Codegen

```rust
emit_enum(&mut out, "Theme",   "THEMES",   &themes);
emit_enum(&mut out, "Grammar", "GRAMMARS", &grammars);
fs::write(out_dir.join("assets_gen.rs"), out)?;
```

`assets_gen.rs` lands in `OUT_DIR`; included in `lib.rs` via:

```rust
include!(concat!(env!("OUT_DIR"), "/assets_gen.rs"));
```

## Generated shape

For each enum:

```rust
pub enum Theme {
    Variant1,
    Variant2,
    /* ... */
}

impl Theme {
    pub const fn name(self) -> &'static str { /* match -> file_stem */ }
    pub fn from_name(s: &str) -> Option<Self> { /* match -> Some / None */ }
}

pub const THEMES: &[Theme] = &[Theme::Variant1, Theme::Variant2, /* ... */];
```

Identifier sanitisation:

| input | identifier |
|-------|-----------|
| `tsx` | `Tsx` |
| `Catppuccin Mocha` | `CatppuccinMocha` |
| `Solarized (dark)` | `SolarizedDark` |
| `42-something` | `N42Something` |

`name()` returns the original file stem verbatim (with spaces and
parens). `from_name(s)` matches against the file stem.

## Collision handling

Two files with stems that sanitise to the same identifier are
deduped: first wins, second is skipped at codegen. Both files still
load at runtime; only the enum exposure is affected.

## Re-run triggers

```rust
println!("cargo:rerun-if-changed=assets/themes-bat");
println!("cargo:rerun-if-changed=assets/grammars-sublime");
```

Cargo re-runs `build.rs` only when those folders change. Adding a
file forces regeneration.

## Why build-time

| approach | trade-off |
|----------|-----------|
| build-time enum codegen | compile-time validated names; cheap zero-cost lookups |
| runtime registry | more flexible (load themes from disk); slower; no type-safe references |

Bundle is closed; embedding via build-time codegen gives consumers
type-safe `Theme::CatppuccinMocha` references.

## Adding a theme

1. Drop `MyTheme.tmTheme` into `dmc-highlight/assets/themes-bat/`.
2. `cargo build` -> build.rs detects the change, regenerates the
   enum, recompiles the crate.
3. New variant `Theme::MyTheme` is available; `Theme::from_name("MyTheme")`
   works.

Same flow for grammars in `assets/grammars-sublime/`.

## Asset format

Themes are TextMate `.tmTheme` files (XML plist). Grammars are
`.sublime-syntax` files (YAML). Both formats are syntect's native
input; the crate ships a converter script
(`scripts/convert-shiki-assets.mjs`) that turns shiki's JSON
grammars into `.sublime-syntax`.

## Bundle source

| folder | source |
|--------|--------|
| `assets/themes-bat/` | curated subset of bat's `themes.bin` |
| `assets/grammars-sublime/` | converted from shiki's grammar JSON |

To refresh:

```bash
cd dmc-highlight
npm i plist
node scripts/convert-shiki-assets.mjs
```

The script writes new `.sublime-syntax` files; commit them, rebuild.
