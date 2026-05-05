# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1](https://github.com/gentleeduck/duck-mc/compare/dmc-diagnostic-v0.1.0...dmc-diagnostic-v0.1.1) - 2026-05-05

### Other

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
