# Bundled syntax-highlighting assets

Source: [shikijs/textmate-grammars-themes](https://github.com/shikijs/textmate-grammars-themes)
(MIT). Mirrors the upstream `tm-themes` + `tm-grammars` packages shiki
itself ships.

## Layout

```
assets/
  themes-json/     65  VS Code .json themes (raw shiki output)
  grammars-json/  253  .tmLanguage.json grammars (raw shiki output)
  themes/              .tmTheme plist (converted, syntect-loadable)  -- TODO
  grammars/            .sublime-syntax / .tmLanguage (plist)         -- TODO
  themes.packdump      serialised SyntaxSet                          -- TODO build.rs
  grammars.packdump    serialised ThemeSet                           -- TODO build.rs
```

## Conversion gap

`syntect` does **not** read VS Code JSON themes or `.tmLanguage.json`
grammars natively. It needs `.tmTheme` (plist XML) and `.sublime-syntax`
(YAML) or `.tmLanguage` (plist XML).

The raw JSON is bundled as the upstream source of truth. A `build.rs`
script (TODO) converts to syntect's accepted formats at build time and
serialises into `.packdump` files for fast runtime load.

## Update workflow

```sh
git clone --depth 1 https://github.com/shikijs/textmate-grammars-themes /tmp/shiki-tmt
rm -rf themes-json grammars-json
mkdir -p themes-json grammars-json
cp /tmp/shiki-tmt/packages/tm-themes/themes/*.json   themes-json/
cp /tmp/shiki-tmt/packages/tm-grammars/grammars/*.json grammars-json/
```

## Licenses

Per-file headers retain upstream license info. Grammars draw from many
upstream sources (VS Code language extensions, Sublime packages, etc),
mostly MIT/BSD/Apache. Themes are mostly MIT (creator-attributed).
Verify per-file header before redistributing as a binary blob if
licensing matters for your build.
