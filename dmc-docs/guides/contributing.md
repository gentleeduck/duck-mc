# Contributing

Workflow notes for adding features. Terse.

## Repo layout

```
@duck-md/
|- dmc-lexer/       crate
|- dmc-parser/      crate
|- dmc-highlight/   crate (leaf, syntect bundle)
|- dmc-transform/   crate
|- dmc-codegen/     crate
|- dmc-diagnostic/  crate
|- dmc-schema/      crate
|- dmc-core/        crate (engine + CLI)
|- dmc-napi/        crate (cdylib + TS wrapper)
|- dmc-sidecar/     pure JS (Node helper)
|- dmc-docs/        these docs
|- examples/        nextjs / nextjs-velite / acme-docs / web
`- docs/            external-facing prose (pre-dmc-docs era)
```

## Build

```bash
# rust workspace
cargo check --workspace
cargo test  --workspace
cargo build --release -p dmc-core --features pretty-code

# napi binary (refresh after Rust changes)
pnpm --filter @duck/md run build

# example apps
pnpm --filter dmc-nextjs dev
pnpm --filter velite-nextjs dev   # for comparison
```

## Add a transformer

See [`../dmc-transform/writing-a-transformer.md`](../dmc-transform/writing-a-transformer.md).
Quick checklist:

1. New file in `dmc-transform/src/builtin/<name>.rs`.
2. `pub use` it in `dmc-transform/src/builtin/mod.rs` under a Cargo
   feature.
3. Mirror the feature in `dmc-transform/Cargo.toml` and
   `dmc-core/Cargo.toml` (forwarding).
4. Register in `Pipeline::with_defaults_for(cfg)`.
5. Add stripped JS plugin name(s) to
   `dmc-core::engine::compile::is_native_owned_*` if there is a JS
   equivalent.
6. Doc under `dmc-docs/dmc-transform/transformers/<name>.md`.
7. Update `dmc-docs/dmc-transform/transformers/index.md` table.

## Add a theme

See [`../dmc-highlight/build-rs.md`](../dmc-highlight/build-rs.md).

1. Drop `MyTheme.tmTheme` into `dmc-highlight/assets/themes-bat/`.
2. Rebuild. `build.rs` regenerates the `Theme` enum.
3. New variant `Theme::MyTheme` is available.

## Add a grammar

1. Drop `MyLang.sublime-syntax` into
   `dmc-highlight/assets/grammars-sublime/`.
2. Or run `node scripts/convert-shiki-assets.mjs` to regenerate the
   bundle from shiki's grammar JSON.
3. Rebuild.

## Add a diagnostic code

In `dmc-diagnostic/src/lib.rs`:

1. Add the variant to the `Code` enum under the matching layer's
   feature gate.
2. Add the canonical id (`T010`, `PW005`, etc) to `Code::code()`.
3. Add severity to `Code::severity()`.
4. Doc under `dmc-docs/dmc-diagnostic/codes.md`.

## Add a CLI flag

In `dmc-core/src/cli/`:

1. Add the clap field.
2. Plumb to `EngineConfig`.
3. Doc under `dmc-docs/guides/cli-reference.md`.

## Tests

```bash
cargo test -p dmc-parser              # parser only
cargo test --workspace --features pretty-code   # full
```

Per-crate `tests/*.rs` for integration; `#[cfg(test)] mod tests`
for unit. Snapshot tests via `insta` in `dmc-core` (compile output
fixtures).

## Bench

```bash
cargo run --release -p dmc-core --features pretty-code --example bench
```

Output: `dmc-core/tmp/bench.json` + SVG plots. See
[`performance.md`](performance.md) for headline numbers.

## Style

- Caveman-mode terse comments / docs / commits. No filler. No
  em-dashes, curly quotes, ellipsis glyphs in source or docs.
- ASCII only in prose. Use `-`, `'`, `"`, `...`, `->`, `<-`, `>=`,
  `<=`, `!=`, `*`, `.`.
- Prefer fragments. Drop articles when natural.
- Comments explain WHY, not WHAT. Identifiers should make the WHAT
  obvious.

## Pre-commit

```bash
cargo fmt
cargo clippy --workspace --all-features -- -D warnings
cargo test  --workspace
```

`rustfmt.toml` at repo root sets max-width 120 + a few tweaks; CI
enforces. clippy must pass with `-D warnings`.

## PR checklist

- [ ] Tests pass: `cargo test --workspace --features pretty-code`
- [ ] Docs updated (per-crate + cheatsheet if API changes)
- [ ] No special chars in prose (em-dash, curly quotes, etc)
- [ ] Bench numbers if perf-relevant change
- [ ] Cache key updated if compile output shape changes (else stale
      caches break consumers)

## Cache key gotcha

The persistent file cache key includes a serde fingerprint of
`(CompileConfig, include_html, collection_name, schema, output_format)`.
Adding a new field to `CompileConfig` that affects rendered output
MUST be included in `Collection::process`'s `cfg_fp` tuple.
Otherwise warm rebuilds serve stale output.

When in doubt, bump `CARGO_PKG_VERSION` (the cache key's outermost
component). Forces every entry to invalidate.
