# Roadmap — Production Hardening

Tracks open items between current state and "ship-grade" production readiness for the dmc compiler + the duck-ui consumer.

## 1. Incremental preMdx mirror cache (perf, not correctness)

**Status:** RESOLVED.

`preprocessMdxIntoMirror` (`dmc-napi/mod.ts`) currently wipes
`<root>/.dmc-cache/preprocessed/` and re-runs every user `preMdxPlugin`
on every `.mdx` file every build. Cache invalidation is sound — dmc's
native per-file cache (`blake3(mirror_content + path + cfg_fingerprint)`)
picks up plugin output changes through the regenerated mirror — so
results stay correct. The cost is wasted CPU on unchanged files.

**Implementation:**

- `<mirror>/.manifest.json` maps
  `relPath → { sourceHash, pluginsHash, extraInputsHash }`.
- `sourceHash` = SHA-256 of source bytes.
- `pluginsHash` = SHA-256 of `fn.toString()` + serialized options for
  every plugin in `preMdxPlugins`. Captures source-level edits to a
  plugin without dragging in a graph-wide module hasher.
- `extraInputsHash` = SHA-256 of every path declared in
  `content.preMdxCacheInputs` (concrete paths, no globs). Missing
  files contribute the empty hash so a fresh checkout doesn't error.
- Cache hit when all three hashes match the previous build AND the
  mirror file still exists; the unified pipeline is skipped for that
  file.
- Stale mirror entries (source removed) get swept after the loop.
- Manifest write is best-effort: failure degrades to "no cache" on
  the next build instead of failing the build.

**Measured (`apps/duck`, 370 mdx files):**

- Cold: `0 hits, 370 misses (1345ms)`.
- Warm: `370 hits, 0 misses (221ms)` — **6.1× faster** preMdx step.
- Single-file edit: `369 hits, 1 miss (205ms)` — touched file
  re-runs, others skip.

**Caveat:** preMdx plugins that read filesystem state outside
`preMdxCacheInputs` won't auto-invalidate. Document the slot's
purpose in the consumer-facing config docs; users always have
`rm -rf .dmc-cache` as the escape hatch.

---

## 2. napi prod loader under Next/Turbopack

**Status:** RESOLVED for `apps/duck`. `next build` no longer fails on
the napi binary — the binary loads at build time only (config files
+ build scripts), so there's no runtime resolution to fix. The
production-build issues that surfaced were a different cluster:

1. Velite-style config registered only `duckUi` even though
   `apps/duck` imports 21 collections + a top-level `docs`. Fixed in
   `velite/config.ts` — every package in the `packages` array now
   gets a collection, plus `docs` for the top-level pattern.
2. Collection `name` was PascalCased (`DuckUi`), but
   `build-search-index.mjs` and `app/**/page.tsx` import camelCased
   identifiers. Aligned: `name = key` (camelCase) so on-disk
   filenames match every consumer.
3. `SchemaBuilder<T>` fluent helpers (`.max`, `.optional`, `.default`,
   …) returned `SchemaBuilder<unknown>`, decaying every chain. Made
   them all return `SchemaBuilder<_T>` so `.transform((data) => …)`
   sees real field types. Fixed downstream "title is unknown" /
   "permalink does not exist on never" type errors across page
   metadata helpers.
4. `velite/utils.ts` imported `@duck-docs/context` (an internal alias
   that only exists inside `packages/duck-docs/`). Switched to the
   public export name `@gentleduck/docs/context`.

**Remaining:** `apps/duck`'s prod build is blocked by item #9, not
this. If/when `serverExternalPackages` becomes necessary it can be
revisited — but right now it isn't.

---

## 9. Fence-leak in dmc lexer/parser

**Status:** RESOLVED. CommonMark §4.5 violation in
`lex_fenced_code` close detection — any line starting with
`count` backticks at column 0 was treated as a close, even
`` ```tsx /Generic/ `` style fence opens with trailing info
strings. That parity-flipped every following fence in the
document, so TypeScript generics inside ` ```ts ... ``` ` blocks
(e.g. `Partial<KeyBindOptions>`) leaked into the JSX-flow lexer
and produced `unterminated expression` / dropped-`{...}`
warnings.

**Fix:** `dmc-lexer/src/lexers/code.rs` — close fence only when
the bytes between the closing backtick run and `\n` are
whitespace (per CommonMark spec).

**Result:** `apps/duck` `next build` produces 945 / 945 static
pages with no diagnostics. The 3 previously failing duck-vim
pages render correctly.

3 failing pages, all in `duckVim`:

- `duck-vim/api/command`
- `duck-vim/api/parser`
- `duck-vim/course/02-first-shortcut`

**Symptom:** content inside a fenced ` ```ts ` block leaks out as
top-level JSX. dmc parser produces both:

- a `CodeBlock` whose `__dmcRaw__` correctly contains the fence body
  text (so the fence DID get captured), and
- separately, a `JsxElement` whose name comes from a TypeScript
  generic that appears INSIDE that fence body.

**Repro source pattern** (every failing page has at least one):

```
new KeyHandler(registry: Registry, …, defaultOptions?: Partial<KeyBindOptions>)
…
modifiers: Array<'ctrl' | 'alt' | 'meta' | 'shift'>
…
const KEY_ALIASES: Record<string, string> = { … }
```

The `<KeyBindOptions>` / `<Record>` / `<Array>` get re-emitted as
`jsxs(KeyBindOptions, …)` etc. in the compiled body, against a
`_components` map that doesn't carry those names — so the consumer's
`useMDXComponent(code)` throws `_missingMdxReference("KeyBindOptions")`
or, more often, a `SyntaxError: Unexpected token ':'` because dmc
nested raw TypeScript fragments into JSX children where they're
parsed as JS.

**Already done this cycle:**

- `imports`/`exports` no longer leak into the function body (codegen
  unconditionally drops the prelude — see `dmc-codegen/src/mdx.rs`
  `into_string`). `new Function(body)` cannot legally contain `import`
  statements, so even if a future parse goes wrong, the runtime never
  blows up on raw ESM. `_search-index.json` build is no longer poisoned
  by stale ESM either.

**Plan:**

- Trace the lexer state through one failing fence end-to-end. The
  first fence in `command.mdx` (`interface Command { … <T>(args?: T) … }`)
  IS captured correctly — so the bug is state corruption between that
  fence and the next, not a generic `<T>` mishandling.
- Likely suspects, by descending odds:
  1. `lex_fenced_code` body loop: confirm `column` resets to 0 on
     every `\n` advance (relevant if a non-ASCII char or multibyte
     edge slips through `advance_bytes`).
  2. Inline backticks in surrounding markdown (`### \`constructor(…)\``
     headings) bleeding into a state that suppresses the next fence
     open.
  3. `lex_jsx_attribute`'s brace counter tripping on `<Zap />` nested
     inside `icon={<Zap />}` and leaving `column`/`line` desynced.
- Add a fixture in `dmc-parser/tests/` from the failing pages so the
  regression is locked in once fixed.

**Workarounds until fixed:**

- Rewrite the 3 failing pages to escape generics inside fences (e.g.
  use `Partial[KeyBindOptions]` pseudo-syntax, or split the line so
  the `<` does not appear in a code block dmc decides to re-walk).
- Or temporarily drop those 3 entries from the duckVim collection
  via a `pattern` exclusion until the parser fix lands.

---

## 3. Theme-bundle diagnostic

**Status:** silent fallback today.

`prettyCode.theme = { light: 'github-light', dark: 'Catppuccin Mocha' }`
silently leaves `--dmc-light` undefined when the theme isn't bundled in
syntect. UI falls back to default-mode color, hard to debug.

**Plan:**

- Enumerate bundled themes once at startup
  (`dmc-highlight::list_bundled_themes()`).
- During `PrettyCode::transform`, if the requested theme isn't in the
  list, emit a `Code::ThemeNotBundled` diagnostic with the available
  set as a hint.
- Expose `defaultMode` fallback explicitly so consumers can choose
  "fail loud" vs "silent fallback".

---

## 4. dist-rebuild trap (`@gentleduck/docs`)

**Status:** documented nowhere.

Editing `packages/duck-docs/src/**` requires `bun run build` in that
package — Next reads `dist/` per the `exports` map. HMR doesn't trigger
the package build. Easy footgun for new contributors.

**Plan:**

- Either: add a `prepare`/watch script that rebuilds `dist/` on save
  (chokidar + tsdown watch), wired into `apps/duck/package.json`'s
  `dev` script.
- Or: change `exports` to point at `src/**` in dev (conditional
  `"development"` export) and `dist/**` in prod.
- Document the chosen path in `packages/duck-docs/README.md`.

---

## 5. Single-file `<ComponentSource path="…/file.tsx" />`

**Status:** untested this cycle.

Multi-file (directory) path verified end-to-end. Single-file path
shares the same React component but `items.length === 1` branch was
never visually checked.

**Plan:**

- Add a docs page that uses `<ComponentSource path="packages/.../accordion/index.ts" />`
  directly.
- Verify: filename label appears, `// filename` line stripped, copy
  button works.
- Add a snapshot test in `dmc-transform/tests/pretty_code.rs` covering
  the single-block injection shape.

---

## 6. ~5-19% bench regression vs phase-5 baseline

**Status:** partially addressed (default strategy flipped to
`Split`); residual cost recorded in
`duck-benchmarks/phase-6-correctness-cache/`. Re-bench
post-flip pending.

`MultiThemeStrategy::Split` is now the default — flipped after the
phase-6 flamegraph showed pretty-code dominating compile time. Split
emits one solid-colour `<pre data-theme="…">` per theme; per-token
style strings shrink by half vs the css-vars `--dmc-{mode}` layout,
reducing JSX-walk and codegen pressure. CssVars stays available as
an opt-in for consumers with >2 themes.

CssVars strategy emits `--dmc-{mode}` declarations in every styled
token's inline `style`, doubling style-string length vs the old Split
strategy. Within noise band, but real.

**Plan:**

- Hoist mode-invariant style fragments (font-weight, font-style,
  text-decoration) out of the per-token `style` and onto a single
  `[data-dmc-fragment] code span` rule in the consumer CSS — only the
  color vars stay inline.
- Re-run bench. Target: <2% vs phase-5.

---

## 7. `preMdxPlugins` type safety

**Status:** RESOLVED.

**Changes:**

- New exported alias
  `PreMdxPlugin<Options> = Plugin<Options, any, any> | [Plugin<Options, any, any>, ...Options]`
  in `dmc-napi/mod.ts`. Strings and nested `PluggableList` are
  excluded — the preMdx pipeline runs each entry directly so neither
  is meaningful.
- `ContentOptions.preMdxPlugins` switched from `Pluggable[]` →
  `PreMdxPlugin[]`.
- Tree generic deliberately left as unified's default (`any`).
  Consumer plugins routinely declare narrower trees
  (e.g. `IUnistTree extends Node & { children }`); a stricter `Node`
  bound here rejects them via contravariance. Tuple options shape is
  the part worth tightening; tree typing belongs in the plugin
  itself.

**Verified:** `bunx tsc` clean on `dmc-napi` and on
`apps/duck/tsconfig.json`. The previously-permissive
`as Pluggable[]` cast inside `build()` removed.

---

## 8. Lowercase JSX tag depth tracking

**Status:** edge case.

`dedentJsxFlowChildren` only matches `^<([A-Z]\w*)…>` so lowercase
flow JSX components (`<my-element>`) skip depth bookkeeping. Rare in
MDX (lowercase = host element, not component) but possible.

**Plan:**

- Extend regex to `^<([A-Za-z][\w-]*)…>` and the matching
  `</...>` close.
- Skip if the tag name is a known host element (whitelist: `div`,
  `span`, `p`, `pre`, `code`, …) so we don't track every host tag.
- Add a fixture in `dmc-transform/tests/pipeline.rs`.

---

## Pre-prod checklist

- [ ] `next build` clean on `apps/duck` (drives item #2).
- [ ] Item #4 resolved one way or the other — current footgun blocks
      onboarding.
- [ ] Item #3 emits diagnostic on bad theme name.
- [ ] Bench re-run after item #6 hoist.
- [ ] Smoke single-file ComponentSource (item #5).
- [ ] Items #1, #7, #8 are nice-to-haves; ship without if needed.
