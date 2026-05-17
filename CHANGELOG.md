# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.7](https://github.com/gentleeduck/duck-mc/compare/dmc-lexer-v0.3.6...dmc-lexer-v0.3.7) - 2026-05-17

### Other

- update Cargo.lock dependencies

## [0.3.6](https://github.com/gentleeduck/duck-mc/compare/dmc-lexer-v0.3.5...dmc-lexer-v0.3.6) - 2026-05-17

### Other

- update Cargo.lock dependencies

## [0.3.5](https://github.com/gentleeduck/duck-mc/compare/dmc-lexer-v0.3.4...dmc-lexer-v0.3.5) - 2026-05-17

### Other

- update Cargo.lock dependencies

## [0.3.4](https://github.com/gentleeduck/duck-mc/compare/dmc-lexer-v0.3.3...dmc-lexer-v0.3.4) - 2026-05-16

### Other

- update Cargo.lock dependencies

## [0.3.3](https://github.com/gentleeduck/duck-mc/compare/dmc-diagnostic-v0.3.1...dmc-diagnostic-v0.3.3) - 2026-05-16

### Other

- normalize em-dash to hyphen in comments and docs
- trim stale doc comments across crates

## [0.3.2](https://github.com/gentleeduck/duck-mc/compare/dmc-diagnostic-v0.3.1...dmc-diagnostic-v0.3.2) - 2026-05-16

### Other

- normalize em-dash to hyphen in comments and docs
- trim stale doc comments across crates

## [0.3.1](https://github.com/gentleeduck/duck-mc/compare/dmc-lexer-v0.3.0...dmc-lexer-v0.3.1) - 2026-05-12

### Other

- update Cargo.lock dependencies

## [0.3.0](https://github.com/gentleeduck/duck-mc/compare/dmc-parser-v0.2.3...dmc-parser-v0.3.0) - 2026-05-12

### Fixed

- *(dmc-parser)* MDX-classed <div> with component children parses as JSX
- *(dmc-codegen,dmc-parser)* inline raw-HTML no longer drops enclosing JSX block
- *(dmc-parser)* JSX close tag of an enclosing element no longer swallowed as text

### Changed

- `dmc-parser`: finished the CommonMark grind and moved the spec
  baseline from `118/652` to `652/652`.
- `dmc-parser`: added explicit dialect flags via
  `ParseOptions::{cm_strict_html_blocks, gfm_autolinks, legacy_gfm_emphasis}`.
- `dmc-codegen`: added `RenderOptions::gfm_disallowed_raw_html` for
  the GFM tagfilter/disallowed-raw-HTML mode.
- `dmc-parser`: finished the GFM push from `0/670` to `670/670`.
- `dmc-parser`: split the block parser into
  `block/{mod,list,blockquote,code,heading,html}.rs`.
- workspace: tightened clippy + rustdoc hygiene, verified
  multi-codepoint HTML entity decoding, and documented the remaining
  Unicode reference-label case-fold approximation.

### Added

- `dmc-parser` Criterion parse benches and recorded baselines in
  [`duck-benchmarks/BENCHMARKS.md`](./duck-benchmarks/BENCHMARKS.md).
- `duck-benchmarks/phase-7-g-hardening/` - re-run of the `dmc-core`
  compile-pipeline bench after the `G1` - `G9` hardening track, plus
  `flame.svg` / `stage-profile.txt` / `duck-ui.svg` flamegraph
  captures (the consumer-corpus one over the real `apps/duck` 370-mdx
  set).
- `dmc-core/examples/flamegraph_consumer` now falls back to the raw
  `apps/duck/content/` tree when the `.dmc-cache/preprocessed` mirror
  hasn't been generated, and reports which corpus + a per-file average
  in `duck-ui.txt`.

### Docs

- Moved `BENCHMARKS.md` into `duck-benchmarks/` alongside the recorded
  phase folders; updated links in `CHANGELOG.md` and
  `dmc-parser/ROADMAP.md`.
- Added [`duck-benchmarks/GUIDE.md`](./duck-benchmarks/GUIDE.md) - how to record a new bench phase and validate signal vs host noise - and linked it from `duck-benchmarks/README.md`.
- Added [`duck-benchmarks/OPTIMIZATIONS.md`](./duck-benchmarks/OPTIMIZATIONS.md) - per-crate catalogue of remaining optimization opportunities (token streaming, alloc-free text, path interning, syntect output caching) with rough estimates, plus a "done wrong due to timeline" debt list. Linked from the main README and `BENCHMARKS.md`.
- Replaced fancy Unicode punctuation (em/en dashes, curly quotes, ellipsis, arrows, `x`/`u`/`section`/`~`/`<=` substitutions) with ASCII across docs and source comments/strings; test fixtures, spec suites, and asset data left untouched.
- Refreshed parser and lexer roadmaps to reflect current spec status.
- Rewrote crate READMEs for `dmc-lexer`, `dmc-parser`,
  `dmc-codegen`, and `dmc-transform` around current public APIs and
  compliance status.

### Removed

- `package-lock.json` (stale npm lockfile; pnpm is canonical) and
  `dmc-napi/bun.lock` (redundant with `dmc-napi/pnpm-lock.yaml`).
- `examples/acme-docs/tsconfig.tsbuildinfo`, `examples/web/tsconfig.tsbuildinfo`
  (accidentally committed TS build artifacts; `*.tsbuildinfo` now gitignored).
- `examples/nextjs-dmc-full/` example app (unreferenced except by the
  historical `dmc-docs/architecture/compiler-gaps.md` write-up, now noted there).
- `dmc-highlight/assets/grammars/` + `assets/themes/` (unused converted
  outputs - `build.rs` and `lib.rs` embed `grammars-sublime/` + `themes-bat/`;
  `assets/README.md` rewritten to match reality).
- `dmc-highlight/examples/highlight_demo.rs` (dev smoke test redundant
  with the crate's tests).

## [0.2.2](https://github.com/gentleeduck/duck-mc/compare/dmc-lexer-v0.2.1...dmc-lexer-v0.2.2) - 2026-05-07

### Other

- update Cargo.lock dependencies

## [0.2.1](https://github.com/gentleeduck/duck-mc/compare/dmc-highlight-v0.2.0...dmc-highlight-v0.2.1) - 2026-05-07

### Fixed

- *(dmc-highlight)* embed grammars + themes via include_dir

## [0.2.0](https://github.com/gentleeduck/duck-mc/compare/dmc-diagnostic-v0.1.0...dmc-diagnostic-v0.2.0) - 2026-05-07

### Other

- standardize root + per-crate README structure
- per-crate READMEs with shared duck-ui logo

### Added

- duck-benchmarks/ phase log with per-phase READMEs and cross-phase
  summary table.
- LICENSE, CONTRIBUTING, CODE_OF_CONDUCT, SECURITY, and CHANGELOG
  scaffolding files.
- dmc-docs/ expanded with per-crate references, architecture
  cross-cuts, and integration guides for Next.js, Astro, Vite,
  SvelteKit, and Remix.

### Changed

- Renamed napi pkg from `@duck/md` to `@gentleduck/md`.
- Renamed sidecar pkg from `@duck/md-sidecar` to
  `@gentleduck/md-sidecar`.

### Fixed

- Parser now supports lists nested inside blockquotes.

### Removed

- Orphaned `dmc-core/core-samples/` fixture (binary already removed).
- Unused `SourceMeta.version` field across 16 init sites.
- Unused `walk_mut` and `preprocess_math_source` aliases.

## [0.1.0] - 2026-05-04

Initial public-ish release.

### Added

- Native Rust pipeline: lexer, parser, transformers, codegen, schema,
  engine.
- Velite-shaped TypeScript API (`defineConfig`, `defineCollection`,
  `defineSchema`, `definePlugin`, `s.*`).
- Bundled syntect grammars and themes (Catppuccin Latte / Mocha pair
  by default).
- Native math rendering via KaTeX (quick-js) and pulldown-latex
  (MathML).
- Persistent file + math caches.
- Plugin gate that strips JS plugins whose work is owned by a native
  transformer (`remark-gfm`, `rehype-pretty-code`, `rehype-katex`,
  `rehype-slug`, `rehype-autolink-headings`, `remark-math`,
  `remark-emoji`, `shiki`, `rehype-mathjax`).
- Optional `@gentleduck/md-sidecar` Node helper for foreign
  remark/rehype plugins.
- CLI: `dmc build`, `dmc dev`, `dmc compile`.
- Side-by-side example apps (Next.js dmc + Next.js velite) for
  parity comparison.

[Unreleased]: https://github.com/gentleeduck/duck-mc/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/gentleeduck/duck-mc/releases/tag/v0.1.0
