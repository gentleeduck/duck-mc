# Bundled syntax-highlighting assets

Source: [shikijs/textmate-grammars-themes](https://github.com/shikijs/textmate-grammars-themes)
(MIT). Mirrors the upstream `tm-themes` + `tm-grammars` packages shiki
itself ships.

## Layout

```
assets/
  themes-json/      65  VS Code .json themes (raw shiki output, source of truth)
  grammars-json/   253  .tmLanguage.json grammars (raw shiki output, source of truth)
  themes-bat/           .tmTheme plist XML  (syntect-loadable; embedded by lib.rs)
  grammars-sublime/     .sublime-syntax YAML (syntect-loadable; embedded by lib.rs)
```

`build.rs` scans `themes-bat/*.tmTheme` and
`grammars-sublime/*.sublime-syntax` and emits `Theme` / `Grammar`
enums into `$OUT_DIR/assets_gen.rs`. At runtime `src/lib.rs` embeds
those two dirs verbatim via `include_dir!`, so the highlighter has no
filesystem dependency. The `*-json/` dirs are NOT embedded; they are
the upstream raw form, kept so the syntect-format dirs can be
regenerated.

## Why two formats

`syntect` does not read VS Code JSON themes or `.tmLanguage.json`
grammars. It needs `.tmTheme` (plist XML) and `.sublime-syntax` (YAML)
or `.tmLanguage` (plist XML). The raw JSON is the upstream source of
truth; `scripts/convert-shiki-assets.mjs` converts a selected subset
into the `themes-bat/` + `grammars-sublime/` forms that get bundled.

## Update workflow

```sh
git clone --depth 1 https://github.com/shikijs/textmate-grammars-themes /tmp/shiki-tmt
rm -rf themes-json grammars-json
mkdir -p themes-json grammars-json
cp /tmp/shiki-tmt/packages/tm-themes/themes/*.json     themes-json/
cp /tmp/shiki-tmt/packages/tm-grammars/grammars/*.json grammars-json/
node ../scripts/convert-shiki-assets.mjs   # refresh themes-bat/ + grammars-sublime/
```

## Licenses

Per-file headers retain upstream license info. Grammars draw from many
upstream sources (VS Code language extensions, Sublime packages, etc),
mostly MIT/BSD/Apache. Themes are mostly MIT (creator-attributed).
Verify the per-file header before redistributing as a binary blob if
licensing matters for your build.
