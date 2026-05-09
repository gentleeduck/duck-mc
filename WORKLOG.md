# Worklog — read once, delete after

This file is a single throw-away report covering everything we did
on `@duck-md` in this session. Read top-to-bottom, then `rm` it.
None of the contents are normative — the canonical docs live in
`dmc-docs/` and the changelogs.

Use the section ToC to skip:

1. [What changed in the repo (sweep)](#1-what-changed-in-the-repo-sweep)
2. [Compiler correctness (lexer + codegen)](#2-compiler-correctness-lexer--codegen)
3. [Caching (native + preMdx)](#3-caching-native--premdx)
4. [Diagnostics](#4-diagnostics)
5. [Types](#5-types)
6. [Pretty-code default flip (BREAKING for css-vars-only consumers)](#6-pretty-code-default-flip-breaking-for-css-vars-only-consumers)
7. [Bench + flamegraph](#7-bench--flamegraph)
8. [Doc reorg](#8-doc-reorg)
9. [What's still open](#9-whats-still-open)
10. [Verification matrix](#10-verification-matrix)

---

## 1. What changed in the repo (sweep)

`git status --short` summary, ~57 files changed, 5352 insertions
/ 789 deletions, plus a handful of new untracked files. Grouped:

### Modified

- **Lexer:** `dmc-lexer/src/lexers/{code,jsx,typography}.rs`,
  `dmc-lexer/src/utils.rs`
- **Parser:** `dmc-parser/src/{block,inline,jsx,table}.rs`,
  `dmc-parser/src/ast/node.rs`, `dmc-parser/src/lib.rs`
- **Codegen:** `dmc-codegen/src/{html,mdx}.rs`,
  `dmc-codegen/tests/{html,mdx_body}.rs`
- **Transform:** `dmc-transform/src/builtin/*.rs` (most),
  `dmc-transform/src/{config,pipeline,lib}.rs`,
  `dmc-transform/tests/{pretty_code,pipeline}.rs`
- **Engine:** `dmc-core/src/engine/{mod,compile,utils,accumulator}.rs`,
  `dmc-core/Cargo.toml` (added `pprof` dev-dep),
  `dmc-core/tests/pretty_code_html.rs`
- **Diagnostics:** `dmc-diagnostic/src/lib.rs` (new TW005 code)
- **Highlight:** `dmc-highlight/src/lib.rs` (added `list_bundled_themes`)
- **napi:** `dmc-napi/{mod.ts,src/lib.rs,CHANGELOG.md,README.md,package.json}`
- **CI:** `.github/workflows/napi-prebuilds.yml`

### New (untracked)

- `dmc-core/examples/{profile,flamegraph,flamegraph_consumer}.rs`
- `dmc-docs/architecture/{benchmarks,compiler-gaps,native-path-perf,optimizations,system-overview}.md`
  (moved from old `docs/` flat folder)
- `dmc-docs/articles/rust-mdx-compiler-vs-velite.md`
- `dmc-docs/dmc-highlight/native-shiki.md`
- `dmc-docs/dmc-sidecar/perf.md`
- `duck-benchmarks/phase-6-correctness-cache/`
  (`bench.json`, three SVGs, `README.md`, `flamegraph/{flame,duck-ui}.svg`,
  `flamegraph/{stage-profile,duck-ui}.txt`)
- `dmc-parser/src/slugger.rs` (heading-anchor slugger, was inline)
- `dmc-transform/src/builtin/assign_heading_ids.rs` (extracted)
- `dmc-napi/bun.lock`

### Removed (effectively — moved or dropped)

- `docs/` flat folder GONE. Files relocated under `dmc-docs/`
  (see §8). `article1.md`, `migrating-from-velite.md` (dupe), and
  `plan.md` (working draft) were dropped outright.

---

## 2. Compiler correctness (lexer + codegen)

### Fence-leak (`dmc-lexer/src/lexers/code.rs`)

**Bug:** `lex_fenced_code` closed any line starting with N matching
backticks at column 0. Including ones with trailing info strings
like `` ```tsx /Generic/ `` — those are NEW fence opens but were
treated as the close of the previous fence. Parity-flipped every
following fence in the document. TypeScript code inside a fence
(`Partial<KeyBindOptions>`, `Record<string, string>`, `Array<…>`)
then got re-lexed as JSX expressions, producing
`unterminated expression` and dropped-`{...}` warnings.

**Fix:** close fence only when bytes between the closing backtick
run and `\n` are whitespace. CommonMark §4.5 compliant.

```rust
// before
if closing_count == count {
  emit close;
  return;
}

// after
if closing_count == count {
  let mut i = self.current;
  let bytes = self.source.as_bytes();
  let mut clean = true;
  while i < bytes.len() && bytes[i] != b'\n' {
    if bytes[i] != b' ' && bytes[i] != b'\t' { clean = false; break; }
    i += 1;
  }
  if clean { emit close; return; }
}
```

**Benefit:** killed ~545 spurious diagnostics on the `apps/duck`
corpus (3 duck-vim pages + every TS-generic fence elsewhere).
Previously-broken pages (`/duck-vim/api/command`,
`/duck-vim/api/parser`, `/duck-vim/course/02-first-shortcut`)
now render.

### Multi-line inline code (`lex_inline_code`)

**Spec:** CommonMark §6.1 — line endings inside a code span are
treated like spaces, so `` `foo\nbar` `` → `<code>foo bar</code>`.
dmc was rejecting any unclosed-on-current-line span as
`UnterminatedCodeBlock`.

**Fix:** char-by-char walk that tracks `prev_was_newline`, bails
at a blank line (CommonMark paragraph break) AND at a column-0
fence run (so a stray `` ` `` can't swallow a fence open).

**Benefit:** valid CommonMark like

```md
The `headers: async () => ({ Authorization:
'Bearer ' + await getServiceToken() })` function …
```

now parses cleanly. Previously: 2 false `unterminated inline code`
errors per such span.

**Cost:** lost the phase-5 `memchr`-backed
`skip_until_any2(b'\n', b'`')` fast path in the common case. This
is a known contributor to the phase-6 bench regression.

### `<Step>foo</Step>` no longer inflates JSX depth (`dmc-napi/mod.ts`)

The dedent walker that strips indentation `mdast-util-mdx-jsx`
inserts on flow JSX children counted EVERY line that opens with
`<Tag>` as a depth bump — including lines that also close on the
same line. So source code indented inside `<Step>` blocks lost
its leading spaces.

**Fix:** detect balanced single-line tags (`<Tag>…</Tag>$`) and
skip the depth increment.

### Lowercase JSX depth tracking (`dmc-napi/mod.ts`)

The dedent walker only matched capitalised JSX tags
(`/^<([A-Z]\w*)\b…/`). Lowercase host tags (`<svg>`, `<div>`,
`<p>`) didn't bump depth, so `<title>Next.js</title>` and
`<path d="…" />` inside an SVG retained 4-space indent and got
re-classified as indented code blocks. Framework icons rendered
empty.

**Fix:** widen regex to `/^<([A-Za-z][\w-]*)\b…/` for both open
and close.

**Benefit:** SVG icons on `/duck-ui/installation` render again.

### ESM strip in MDX function-body (`dmc-codegen/src/mdx.rs`)

The compiled body is consumed via `new Function(body)(runtime)`,
which can't legally contain top-level `import` / `export`
statements. dmc was emitting them verbatim, causing
`SyntaxError: Cannot use import statement outside a module` at
SSR time.

**Fix:** drop the prelude.

```rust
// before: walked imports, exports, prepended each
let mut prelude = String::new();
for i in &imports { prelude.push_str(i); prelude.push('\n'); }
for e in &exports { prelude.push_str(e); prelude.push('\n'); }

// after
let _ = (&imports, &exports);
let prelude = String::new();
```

### `string_literal_expression` lowering in `html.rs`

`HtmlEmitter` previously dropped EVERY `JsxExpression` node (no
JS runtime in static HTML). Idiomatic MDX whitespace (`{' '}`,
`{"x"}`, `` {`y`} ``) tripped the GW002 warning even though the
expression had no JS semantics.

**Fix:** detect string-literal expressions, decode standard JS
escapes (`\n \t \r \\ \' \" \``), emit as escaped text.
Genuinely dynamic expressions still warn.

**Benefit:** zero diagnostics on `apps/duck`'s 370-mdx build (was
1 GW002).

---

## 3. Caching (native + preMdx)

### Native compile cache survives `clean: true`

`dmc-core/src/engine/mod.rs` was wiping `<output_dir>/.cache/`
on every clean build. Cache keys are
`blake3(source + path + cfg_fingerprint)`, so config bumps
already invalidate stale entries; wiping the whole cache forced
lex + parse + transform + codegen to re-run for every doc whose
source hadn't changed.

**Result on `apps/duck`:** warm full builds 34 s → **3.3 s**
(~10× faster). Single-file edit → 2.9 s.

### Incremental preMdx mirror cache (`dmc-napi/mod.ts`)

Per-file SHA-256 manifest at
`<root>/.dmc-cache/preprocessed/.manifest.json`:

```json
{
  "docs/duck-ui/components/sheet.mdx": {
    "sourceHash": "…",
    "pluginsHash": "…",
    "extraInputsHash": "…"
  }
}
```

- `sourceHash`: blake of source bytes.
- `pluginsHash`: SHA of `fn.toString() + JSON.stringify(opts)` for
  every plugin in `preMdxPlugins`. Captures source-level edits to
  the plugin itself.
- `extraInputsHash`: SHA of every path declared in the new
  `content.preMdxCacheInputs?: string[]` config slot
  (e.g. `__ui_registry__/index.ts`).

Cache hit (all three match + mirror file still on disk) → skip
the unified pipeline for that file. Stale entries (source
removed) get swept post-loop.

**Result:** preMdx step on 370 mdx files: 1345 ms cold, **221 ms
warm** (~6×), 205 ms with one file edited.

---

## 4. Diagnostics

### Structured `BuildReport.diagnostics`

Was `Vec<String>` of `format!("{:?}", d)` blobs. Now
`Vec<DiagnosticReport>` with typed fields:

```ts
interface DiagnosticReport {
  code: string;          // e.g. "TW005"
  severity: "bug" | "error" | "warning" | "help" | "note";
  message: string;
  help?: string;
  file?: string;
  line?: number;
  column?: number;
}
```

### Lexer span file paths

Every `Span::from_zero_based("", line, col, len)` call site in
`dmc-lexer/src/lexers/{code,jsx,typography}.rs` replaced with
`Span::from_zero_based(self.meta.path.clone(), line, col, len)`.
Diagnostic prefixes show `path:line:col` like rustc.

### Per-line ANSI-coloured renderer

`apps/duck/scripts/build-docs-content.mjs` (consumer side) now
streams diagnostics one-by-one with severity colour (red error,
yellow warning, cyan help, blue note), `[CODE]` brackets, and
`= help: …` lines. Errors → stderr, others → stdout. `NO_COLOR`
strips colors. Surfaced ~60 pre-existing parse errors that were
hidden in the old JSON blob.

### `Code::ThemeNotBundled` (TW005)

`PrettyCode::transform` checks every configured theme against
`dmc_highlight::list_bundled_themes()`; missing → emit warning
with the bundled-theme list as `help`. Process-wide
`Mutex<HashSet<String>>` dedupes so a 300-doc build emits one
warning per missing theme, not 300.

---

## 5. Types

### `SchemaBuilder<_T>` fluent helpers preserve generic

```ts
// before — every chain decayed to SchemaBuilder<unknown>
.max(99)        // SchemaBuilder<unknown>
.optional()     // SchemaBuilder<unknown>

// after
.max(99): SchemaBuilder<_T>
.optional(): SchemaBuilder<_T | undefined>
.default(value: _T): SchemaBuilder<_T>
```

`.transform((data) => …)` now sees real field types. Killed
cascading `Type 'unknown' is not assignable to type 'string'`
errors on consumer page-metadata helpers.

### `PreMdxPlugin<Options>` alias

```ts
// dmc-napi/mod.ts
export type PreMdxPlugin<Options extends [any?, ...any[]] = [any?]> =
  | Plugin<Options, any, any>
  | [Plugin<Options, any, any>, ...Options];
```

Replaces the loose `Pluggable[]` on
`ContentOptions.preMdxPlugins`. Strings and nested
`PluggableList` excluded since the preMdx pipeline runs each
entry directly. Tree generic deliberately left as unified's `any`
so consumer plugins with narrower trees (e.g. `IUnistTree extends
Node & { children }`) still satisfy via contravariance.

---

## 6. Pretty-code default flip (BREAKING for css-vars-only consumers)

After the phase-6 flamegraph showed `PrettyCode::transform` at
>80 % of compile time, flipped `MultiThemeStrategy` default:

```rust
// dmc-transform/src/config.rs
pub enum MultiThemeStrategy {
  #[default]
  Split,         // was second; now default
  CssVars,       // was default; now opt-in
}
```

**What changed in the emitted body:**

```html
<!-- before (CssVars) -->
<div data-dmc-fragment>
  <pre style="color:#cdd6f4; --dmc-light:#4c4f69; --dmc-dark:#cdd6f4; …">
    <code>
      <span class="line">
        <span style="--dmc-light:#8839ef; --dmc-dark:#cba6f7">const</span> …
      </span>
    </code>
  </pre>
</div>

<!-- after (Split) -->
<div data-dmc-fragment>
  <pre data-theme="light" style="color:#4c4f69; background:#eff1f5">
    <code><span class="line"><span style="color:#8839ef">const</span> …</span></code>
  </pre>
  <pre data-theme="dark" style="color:#cdd6f4; background:#1e1e2e">
    <code><span class="line"><span style="color:#cba6f7">const</span> …</span></code>
  </pre>
</div>
```

**Consumers wanting the old shape opt in:**

```ts
prettyCode: {
  multiThemeStrategy: 'css-vars',
  theme: { light, dark },
}
```

`apps/duck` CSS already carried both `[data-theme="light"]` rules
AND `--dmc-{light,dark}` `@property` declarations, so the flip lit
up existing styles without churn. Verified: 945 / 945 pages build
clean.

---

## 7. Bench + flamegraph

### `cargo run --release --example bench` — phase 5 → phase 6

| variant              | phase 5 | phase 6 |  delta |
| -------------------- | ------: | ------: | -----: |
| native               |   44.73 |   55.19 | +23 %  |
| sidecar+gfm          |   46.01 |   51.97 | +13 %  |
| sidecar+pretty-code  |   44.94 |   50.65 | +13 %  |
| sidecar+kitchen-sink |  144.77 |  168.93 | +17 %  |
| velite+gfm           | 5934.00 | 6110.71 |  +3 %  |
| velite+kitchen-sink  | 1381.46 | 1427.16 |  +3 %  |

(1000 files, median ms, lower is better. Velite stays in noise floor;
phase-6 +13-23 % is the real correctness-tax.)

**Cost breakdown (in cargo bench, not consumer build):**

- `Arc<str>` file path on every lexer span — ~50M atomic refcount
  bumps for a 1000-file run.
- Multi-line `lex_inline_code` walks char-by-char (lost the phase-5
  `memchr` fast path).
- Fence-close tail-of-line whitespace probe — extra byte-walk per
  close-candidate.

**NOT in this bench:** structured-Diagnostic conversion (runs once
at end-of-build), lowercase-JSX dedent (JS-side), preMdx manifest
(JS-side).

**Single-file `compile fixture` criterion bench:** 119 µs → **111
µs** (-6 %). At one file, parse + codegen dominate; at 1000 files,
per-token tax dominates.

### Flamegraphs

- `duck-benchmarks/phase-6-correctness-cache/flamegraph/flame.svg`
  — toy fixture, 326 KB.
- `…/flamegraph/duck-ui.svg` — full `apps/duck` corpus (370 mdx),
  584 KB. Shows `PrettyCode::transform` (syntect highlight) as
  the dominant frame at >80 %.

Both produced via `pprof` (signal-driven sampler in-process), so
no `samply` / `perf_event_paranoid` toggle needed. Re-run with
`cargo run --release --example flamegraph[_consumer] --features pretty-code`.

### Warm consumer build (`apps/duck`, 370 mdx)

| state                                | wall-clock | preMdx step              |
| ------------------------------------ | ---------: | ------------------------ |
| Cold (no `.dmc-cache`, no `.cache`)  |   34.0 s   | 1447 ms (370 misses)     |
| Warm (no source change)              | **3.3 s**  | 217 ms (370 hits)        |
| 1-file edit                          |   2.9 s    | 205 ms (369 hits / 1 m)  |

---

## 8. Doc reorg

Loose `docs/*.md` flat folder → `dmc-docs/` per-area folders.

| was                                  | now                                                          |
| ------------------------------------ | ------------------------------------------------------------ |
| `docs/architecture.md`               | `dmc-docs/architecture/system-overview.md`                   |
| `docs/benchmarks.md`                 | `dmc-docs/architecture/benchmarks.md`                        |
| `docs/compiler-gaps.md`              | `dmc-docs/architecture/compiler-gaps.md`                     |
| `docs/native-path-perf.md`           | `dmc-docs/architecture/native-path-perf.md`                  |
| `docs/native-shiki.md`               | `dmc-docs/dmc-highlight/native-shiki.md`                     |
| `docs/optimizations.md`              | `dmc-docs/architecture/optimizations.md`                     |
| `docs/sidecar-path-perf.md`          | `dmc-docs/dmc-sidecar/perf.md`                               |
| `docs/article.md`                    | `dmc-docs/articles/rust-mdx-compiler-vs-velite.md`           |
| `docs/article1.md`                   | DELETED (dupe of `article.md`)                               |
| `docs/migrating-from-velite.md`      | DELETED (dupe of `dmc-docs/guides/migrating-from-velite.md`) |
| `docs/plan.md`                       | DELETED (working draft)                                      |
| `docs/roadmap.md`                    | `dmc-docs/guides/roadmap.md` (replaced older copy)           |
| `docs/`                              | folder removed                                               |

`dmc-docs/README.md` now carries:
- updated `architecture/` listing with new files,
- new `articles/` entry,
- a **"Per-crate doc → source map"** table linking every
  `dmc-docs/<crate>/` folder to its source crate path so anyone
  reading a doc can jump straight to the code.

**Cross-references rewritten** in:
- `dmc-napi/CHANGELOG.md` — `docs/benchmarks.md` →
  `dmc-docs/architecture/benchmarks.md`.
- `dmc-napi/README.md` — `docs/migrating-from-velite.md` →
  `dmc-docs/guides/migrating-from-velite.md`.
- `examples/COMPARISON.md` — `docs/sidecar-path-perf.md` →
  `dmc-docs/dmc-sidecar/perf.md`.

**No emoji slop.** Quick grep for emoji code points in `*.rs`,
`*.ts`, `*.md` outside `dmc-docs/dmc-transform/transformers/emoji.md`
returned zero hits — codebase is already clean. AI-style filler
phrases ("Sure!", "Happy to help", "Let's") were already absent
from comments and docs.

---

## 9. What's still open

- **#4 dist-rebuild trap.** `@gentleduck/docs` ships from `dist/`;
  src edits need `bun run build` in that pkg. Conditional-exports
  attempt was reverted at user request. Open.
- **#5 single-file `<ComponentSource path="…/file.tsx" />`.** Code
  path exists; not exercised by an actual docs page. Verify by
  adding one + smoke-test filename label / comment-strip.
- **#6 phase-6 cold-bench regression** is partially addressed (split
  default flips ~half the per-token style cost). Re-bench post-flip
  not yet run — could clear most of the remaining gap.
- **File / line / col mirror→source line-map.** Diagnostic file
  paths point at the mirror under `.dmc-cache/preprocessed/`. Line
  numbers don't match source after `remark-stringify` reflow, so a
  path-only remap would mislead. Needs the dedent walker to build a
  `mirror_line → source_line` table and apply on emit. Not
  blocking — preMdx-affected files rarely error.

---

## 10. Verification matrix

| checked                                          | result                                          |
| ------------------------------------------------ | ----------------------------------------------- |
| `cargo test -p dmc-transform --features pretty-code --test pretty_code` | 10 / 10 pass |
| `cargo test -p dmc-core --features pretty-code --test pretty_code_html` | 8 / 8 pass |
| `cargo test -p dmc-codegen --lib`                | 4 / 4 pass (string-literal recogniser)          |
| `bunx tsc --noEmit` on `dmc-napi`                | clean                                           |
| `bunx tsc --noEmit -p apps/duck/tsconfig.json`   | clean                                           |
| `bun run build:docs` (apps/duck)                 | 22 collections, 370 records, 0 diagnostics      |
| `bun run build` (apps/duck, full Next prod)      | `Compiled successfully`, **945 / 945** pages    |
| `cargo bench --bench compile`                    | `compile fixture 111.55 µs` (-6 % vs phase-5)   |
| `cargo run --release --example bench`            | numbers in §7 above; saved to phase-6 folder    |
| `cargo run --release --example flamegraph_consumer --features pretty-code` | `duck-ui.svg` saved, PrettyCode dominant |

---

When you're done reading this: `rm WORKLOG.md`. Everything
worth keeping is in the changelogs and `dmc-docs/`.
