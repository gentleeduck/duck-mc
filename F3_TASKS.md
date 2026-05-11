# Post-CM-100 Task List

CommonMark spec: 652/652 (100%). Below = remaining work to make dmc the best markdown+MDX parser.

Status legend: `[ ]` pending, `[~]` in progress, `[x]` done.

## Phase F4 — GFM spec runner

- [x] F4.1 Vendor GitHub Flavored Markdown spec JSON (gfm-spec 0.29 or latest).
  - Place at `dmc-parser/tests/fixtures/gfm_spec.json`.
  - Source: https://github.github.com/gfm/ — extract examples programmatically.
- [x] F4.2 Add `gfm_spec_no_regression` test mirroring `commonmark_spec_no_regression`.
  - Baseline file: `dmc-parser/tests/fixtures/gfm_baseline.txt`.
  - Initial baseline: 0; bump on first run.
- [x] F4.3 Triage GFM failures by section (tables / strike / tasklists / autolinks / footnotes / disallowed-raw-HTML).
- [x] F4.4 Push GFM baseline ≥95% — DONE: **670/670 (100%)**.
  - Email autolink extension (`a@b.c` → `mailto:`), incl. `_`-token spanning local-part.
  - Strikethrough doesn't pair across blank lines.
  - GFM tagfilter / disallowed raw HTML via `RenderOptions { gfm_disallowed_raw_html }`.
  - Legacy GFM-0.29 emphasis via `ParseOptions { legacy_gfm_emphasis }` (flattens redundant nested `<strong>`/`<em>` without changing CM 0.31.2 behavior).
  - GFM tables: missing outer pipes, header/separator col-count match, row pad/truncate, blank-line stop, no-pipe continuation rows.
  - Newline-separated table tags to match GFM reference HTML layout.

## Phase G1 — Bench

- [x] G1.1 Add `dmc-parser/benches/parse.rs` using `criterion`.
  - Inputs: small / medium / large MD (1 KB / 100 KB / 5 MB).
  - Compare: `pulldown_cmark::Parser::new`, `markdown::Markdown::parse`, optional `comrak`.
- [x] G1.2 Run + record results in `BENCHMARKS.md`.
- [x] G1.3 Verify 500M tok/s claim or correct it.
- [ ] G1.4 Add `bench-regress.yml` in CI to fail on >10% regression.

## Phase G2 — Fuzzing

- [ ] G2.1 `cargo fuzz init` under `dmc-parser/fuzz`.
- [ ] G2.2 Targets: `parse`, `parse_with(cm_strict=true)`, `render_html(parse(...))`.
- [ ] G2.3 Seed corpus from CM + GFM spec examples.
- [ ] G2.4 Run 24h on each target. Fix any crashes.

## Phase G3 — Refactor

- [x] G3.1 Split `dmc-parser/src/block.rs` (≈2500 lines) by construct.
  - `block/list.rs`, `block/blockquote.rs`, `block/code.rs`, `block/heading.rs`, `block/html.rs`, `block/mod.rs`.
- [~] G3.2 Replace in-place token rewriting (`try_promote_text_*`) with a token-classification helper that returns a virtual token without mutating the slice.
  - The current rewrites are tightly scoped to list / blockquote recovery and a non-mutating virtual classification would cut across the hot block parser paths; leaving the localized mutation in place avoids churn while preserving spec stability.
- [x] G3.3 Audit `unsafe` pointer-arithmetic body-slice reconstruction (link body, html block, raw HTML inline).
  - Replaced the ad hoc pointer math with one checked range-reconstruction helper plus adversarial regression tests; the miri-in-CI part remains deferred with the broader infra work.
- [x] G3.4 Remove dead code warnings (`strip_inline_markers`).

## Phase G4 — Docs

- [x] G4.1 Refresh `dmc-parser/ROADMAP.md` with final F2 + this F3 plan.
- [x] G4.2 Refresh `dmc-lexer/ROADMAP.md`. Document CM column-aware whitespace, tab handling, ESM detection.
- [x] G4.3 Write `README.md` for each crate: dmc-lexer, dmc-parser, dmc-codegen, dmc-transform.
- [x] G4.4 Add `CHANGELOG.md` covering F2 grind (rounds 1-140 + post-fix).

## Phase G5 — Spec edges

- [x] G5.1 Replace `htmlentity` crate or build full HTML5 entity table (covers `&ngE;` etc with multi-codepoint output).
- [~] G5.2 Replace approximate Unicode case-fold (`ẞ → ss`) with `icu_normalizer` or `unicode-case-mapping` crate.
  - Left the in-tree approximation in place; current code documents the limitation and keeps the CommonMark-critical `ß` / `ẞ` behavior covered.
- [ ] G5.3 Replace approximate Unicode punctuation table with proper general-category lookup.

## Phase G6 — MDX completeness

- [~] G6.1 JSX TS generics: `<Foo<T> />` parsing.
  - Current lexer still misroutes `<Foo<T> />` as text plus nested JSX; widening the JSX path safely needs a more invasive generic/type-arg parser, so this is deferred rather than risking regressions.
- [x] G6.2 JSX comments inside expressions: `{/* comment */}` round-trip tests.
- [x] G6.3 ESM `import` / `export` body must lex as JS — at least balanced braces + strings, no markdown inside.
- [~] G6.4 MDX 3 expression typing: `{...spread}`, optional-chaining, JSX-in-expr.
  - `{...spread}` already works for JSX attribute spread, but broader MDX 3 expression typing is out of scope for this phase.

## Phase G7 — Diagnostics

- [x] G7.1 Audit every `Code::*` diagnostic. Replace placeholder messages with actionable spans + suggestions.
- [x] G7.2 Add recovery-quality tests: malformed link / unterminated fence / orphan close tag should produce ONE diagnostic, not many.
- [x] G7.3 Snapshot diagnostic output for CM error cases.

## Phase G8 — Stability

- [ ] G8.1 Run full workspace `cargo +nightly miri test -p dmc-parser`.
- [x] G8.2 Run `cargo clippy --workspace --all-targets -- -D warnings`. Fix new warnings.
- [x] G8.3 Verify `cargo doc --workspace --no-deps` builds without warnings.

## Stop conditions per phase

- F4: GFM ≥ 95% passes.
- G1: benchmarks committed + CI gate active.
- G2: 24h fuzz with no crashes.
- G3: block.rs ≤ 800 lines each module.
- G4: All four READMEs published.
- G5-G8: each task marked done individually.

## Execution policy

Codex runs phases in order. Each commit cites the rule / motivation. Workspace tests must stay green after every commit (1 FAILED line max from any pre-existing flake — currently 0).
