# @gentleduck/md

## Unreleased

### Compiler correctness

- **CommonMark section 4.5 fence-close compliance** (`dmc-lexer/src/lexers/code.rs`).
  `lex_fenced_code` previously closed any line starting with N matching
  backticks at column 0, including ones with trailing info strings like
  `` ```tsx /Generic/ ``. That parity-flipped every following fence so
  TypeScript generics inside code blocks (`Partial<KeyBindOptions>`,
  `Record<string, string>`, `Array<...>`) leaked into the JSX-flow lexer
  and produced `unterminated expression` / dropped-`{...}` warnings.
  Close fence now requires only whitespace after the backtick run. Killed
  ~545 spurious diagnostics on a real docs corpus.
- **CommonMark section 6.1 multi-line inline code** (`lex_inline_code`).
  Spans now cross newlines per spec; line endings inside are treated like
  spaces. Bails at a blank line and at a column-0 fence run so a stray
  un-closed `` ` `` cannot swallow the rest of the document.
- **`<Step>foo</Step>` no longer inflates JSX depth** in the dedent walker
  (`dmc-napi/mod.ts`). Single-line balanced tags are detected before
  the depth counter increments, so source indent inside fenced code
  inside multi-level JSX wrappers is preserved.
- **Lowercase JSX depth tracking** in the dedent walker. `<svg>`,
  `<div>`, `<p>` and other host tags now bump depth correctly. SVG
  `<title>` / `<path>` no longer get re-classified as 4-space indented
  code blocks; framework icons render again.
- **ESM `import` / `export` dropped from function-body MDX output**
  (`dmc-codegen/src/mdx.rs`). The compiled body is consumed via
  `new Function(body)(runtime)` which cannot legally contain top-level
  `import` statements; emitting them caused
  `SyntaxError: Cannot use import statement outside a module` at SSR
  time.

### Caching

- **Native compile cache survives `clean: true`**
  (`dmc-core/src/engine/mod.rs`). Cache keys are
  `blake3(source + path + cfg_fingerprint)`, so a config bump already
  invalidates stale entries - the previous unconditional
  `remove_dir_all(.cache)` on every clean build was forcing
  lex+parse+transform+codegen to re-run for every doc whose source
  hadn't changed. Result: warm full builds on `apps/duck` go from
  34 s -> 3.3 s (~10x faster).
- **Incremental preMdx mirror cache** (`dmc-napi/mod.ts`,
  `preprocessMdxIntoMirror`). Per-file SHA-256 manifest at
  `<root>/.dmc-cache/preprocessed/.manifest.json` keyed on
  `(sourceHash, pluginsHash, extraInputsHash)`. Cache hit reuses the
  mirror file and skips the unified pipeline for that file. Stale
  entries (source removed) get swept post-loop. New
  `content.preMdxCacheInputs?: string[]` declares concrete extra files
  that gate the cache (e.g. `__ui_registry__/index.ts`). PreMdx step
  measured 1345 ms -> 221 ms warm on 370 mdx files (~6x).

### Diagnostics

- **Structured `BuildReport.diagnostics`**
  (`dmc-napi/src/lib.rs`, `mod.ts`). Was `string[]` of `Debug` blobs;
  now `DiagnosticReport[]` with `{ code, severity, message, help, file,
  line, column }`. Consumers can pretty-print one-by-one with their own
  colors.
- **Lexer span file paths populated** from `meta.path` instead of
  `""` across `dmc-lexer/src/lexers/{code,jsx,typography}.rs`. Diagnostic
  prefixes now show `path:line:col` like rustc.
- **`Code::ThemeNotBundled` (TW005)**. `PrettyCode::transform` warns
  when a configured theme isn't in the bundled syntect set, listing all
  ~20 bundled themes as a hint. Process-wide
  `Mutex<HashSet<String>>` dedupe so a 300-doc build emits one warning
  per missing theme, not 300. New `dmc_highlight::list_bundled_themes()`
  helper backs the check.

### Types

- **`SchemaBuilder<_T>` fluent helpers preserve the generic**.
  `.max() / .min() / .optional() / .nullable() / .default() / ...`
  previously returned `SchemaBuilder<unknown>`, decaying every chain.
  `.transform((data) => ...)` now sees real field types - fixed the
  cascading `Type 'unknown' is not assignable to type 'string'` errors
  on consumer page-metadata helpers.
- **`PreMdxPlugin<Options>`** alias replaces the loose `Pluggable[]` on
  `ContentOptions.preMdxPlugins`. Strings + nested `PluggableList` are
  excluded since the preMdx pipeline runs each entry directly. Tree
  generic deliberately left as unified's `any` so consumer plugins with
  narrower trees (e.g. `IUnistTree`) still satisfy the constraint.

### Pretty-code defaults (BREAKING for consumers without `[data-theme]` CSS)

- **`MultiThemeStrategy::Split` is now the default**, demoting
  `CssVars` to opt-in. Phase-6 flamegraph (`apps/duck` 370-mdx
  corpus) confirmed `PrettyCode::transform` as the dominant CPU
  share at >80 % of compile time. The css-vars strategy compiled
  every token against both `light` and `dark` themes and emitted
  `--dmc-{mode}` / `--dmc-{mode}-bg` pairs in every inline style;
  the split strategy emits one solid colour per token per pre and
  consumes ~half the per-token style-string size. Net effect:
  smaller compiled body, less tokenisation work in code paths that
  walk the JSX tree, and the on-disk shape matches velite +
  rehype-pretty-code byte-for-byte (consumers who were already
  styled for `[data-theme]` need no CSS changes).
- Consumers that explicitly want the single-pre + CSS-vars layout
  (e.g. >2 themes, media-query / class-toggle theme switching
  without re-rendering the code surface) opt in via:
  ```ts
  prettyCode: { multiThemeStrategy: 'css-vars', theme: { light, dark } }
  ```
- `apps/duck` consumer CSS already carries both
  `[data-theme="light"]` rules AND `--dmc-{light,dark}` `@property`
  declarations, so the default flip lights up existing styles
  without churn. Verified: 945 / 945 pages build clean, sheet docs
  emit two `<pre>` per fence with `data-theme="light"` /
  `data-theme="dark"` and solid token colours.

### Benchmarks (criterion, native release)

Re-measured after the lexer / cache changes; same Linux box, same
fixtures as `dmc-docs/architecture/benchmarks.md`. No regression vs the phase-5
baseline despite the fence-close, multi-line-inline, span-file, and
lowercase-JSX additions.

| Bench               | Pre-fix (phase-5) | Now             | Delta    |
| ------------------- | ----------------- | --------------- | -------- |
| `compile fixture`   | 119 us            | **111.55 us**   | -6.3 %   |
| `compile simple`    | -                 | 4.92 us         | new row  |
| `parse fixture`     | -                 | 2.18 us         | new row  |

End-to-end (`cargo run --release --example bench`, 1000 files,
median ms; full numbers + raw samples in
`duck-benchmarks/phase-6-correctness-cache/bench.json`):

| variant              | phase-5 | phase-6 |  delta |
| -------------------- | ------: | ------: | -----: |
| native               |   44.73 |   55.19 | +23 %  |
| sidecar+gfm          |   46.01 |   51.97 | +13 %  |
| sidecar+pretty-code  |   44.94 |   50.65 | +13 %  |
| sidecar+kitchen-sink |  144.77 |  168.93 | +17 %  |
| velite+gfm           | 5934.00 | 6110.71 |  +3 %  |
| velite+kitchen-sink  | 1381.46 | 1427.16 |  +3 %  |

Velite stays within ~3 % (the noise floor on this host). Phase-6
overhead lands in the 13-23 % band - see
`duck-benchmarks/phase-6-correctness-cache/README.md` for the cost
breakdown (`Arc<str>` span paths, multi-line `lex_inline_code`,
fence-close whitespace probe).

End-to-end consumer build (`apps/duck`, 370 mdx, full
`bun run build:docs`):

| Build state                         | Wall-clock | preMdx step                        |
| ----------------------------------- | ---------- | ---------------------------------- |
| Cold (no `.dmc-cache`, no `.cache`) | 34.0 s     | 1447 ms (370 misses)               |
| Warm (no source change)             | **3.3 s**  | 217 ms (370 hits)                  |
| 1-file edit                         | 2.9 s      | 205 ms (369 hits / 1 miss)         |

The 10x warm speedup comes from the native cache surviving
`clean: true` (previously wiped on every build). The preMdx
manifest accounts for the per-step 6x drop on top of that.

## 0.2.2

### Patch Changes

- e29036e: Populate `BuildReport.collections` in the napi `build()` binding. The
  Rust side previously returned only `{ diagnostics }`, but the JS
  wrapper in `mod.js` iterates `report.collections` to run the in-process
  unified pipeline whenever the user config supplies remark/rehype
  plugins. With the field missing, any consumer that passes plugins
  (e.g. `@gentleduck/docs`'s default config) crashed with
  `TypeError: report.collections is not iterable`.

  The binding now reports `{ name, outputPath, records }` for every
  collection plus an `errors` array, matching the shape the JS side
  expects.

## 0.2.1

### Patch Changes

- 721da9e: Embed grammar + theme assets directly into the napi `.node` binary using
  `include_dir!`. Previously `dmc-highlight` loaded grammars and themes via
  `SyntaxSet::load_from_folder(env!("CARGO_MANIFEST_DIR")/...)`, which baked
  the build-time absolute path into the compiled binary. On any machine
  that wasn't the CI runner the path didn't exist, syntect panicked with
  `load grammars-sublime: WalkDir(...) NotFound`, and `native.build`
  returned a partial report - making `report.collections is not iterable`
  appear in callers like `apps/duck`.

## 0.2.0

### Minor Changes

- 37bd35c: Initial npm release wired through changesets + napi-prebuilds. Bumps the
  package to 0.2.0 to track the underlying Rust crates and ships prebuilt
  `.node` binaries for 13 napi-rs canonical targets (macOS x64+arm64,
  Windows x64/x86/arm64, Linux gnu+musl on x64/arm64/armv7, Android
  arm64/armv7, FreeBSD x64).
