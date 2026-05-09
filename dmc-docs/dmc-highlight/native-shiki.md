# Native syntax highlighting via `syntect`

Replace the sidecar's shiki + rehype-pretty-code plugin chain with an
in-process Rust syntax highlighter. Removes the single biggest source of
plugin-path cost. After this lands, sidecar+kitchen-sink should beat
velite+kitchen-sink at N=1000 (target: ~600 ms vs velite's 1,398 ms).

## Why `syntect` (not `two-face`)

`two-face` exists. It wraps `syntect` and ships a curated VS Code theme
+ grammar bundle. **Skipping it** in favour of `syntect` direct.

| crate | maturity | usage | shape |
|-------|----------|-------|-------|
| `syntect` | ~10 years, ~6.5k stars | bat (50k stars), mdbook, lapce | the actual highlighter, textmate engine, battle-tested |
| `two-face` | ~2 years, ~50 stars | hobby projects | thin wrapper over `syntect` with bundled VS Code themes |

`two-face`'s only real value is "ship a theme bundle for me". That's
~30 lines of init code in `syntect` direct. Trading 30 lines for a
mature dep with a 10-year track record is the right call.

If a `syntect` constraint shows up, swap to `two-face` later (same
underlying engine; ~1 day migration).

## Why now

Bench numbers from `dmc-core/tmp/bench.json`:

- sidecar+pretty-code (shiki only) @N=1000: **855 ms**
- sidecar+kitchen-sink @N=1000: **2,666 ms**
- per-file delta vs the lightest sidecar variant: ~2 ms / file
- ~80% of that delta is shiki/textmate parsing in JS

Shiki is the only plugin where Rust has a credible direct replacement
(`syntect` -- textmate grammars + theme support, exact same input
format shiki consumes). Other plugins in the kitchen-sink chain (math,
emoji, slug, autolink) have Rust equivalents but smaller wins.

## What `syntect` provides

- Loads `.tmTheme` files (TextMate / Sublime / VS Code themes; same
  format shiki consumes after a one-time conversion).
- Loads `.sublime-syntax` and `.tmLanguage` grammars.
- Tokenises source via the textmate scope-pattern engine, backed by the
  oniguruma regex binding (`onig` crate).
- Returns ranges with style attributes (foreground, background,
  font-style).
- Default `default-fancy` feature ships a small bundle of themes +
  grammars (Solarized, Monokai, ~30 popular languages).

What it does NOT provide:
- A theme/grammar bundle the size of shiki's. We bundle popular VS Code
  themes (github-dark, github-light, dracula, one-dark-pro,
  vitesse-dark) in `dmc-codegen/assets/themes/` and grammars in
  `dmc-codegen/assets/grammars/`.
- The `rehype-pretty-code` line-annotation syntax (`{1,3-5}`,
  `// [!code highlight]`, etc).
- The `title=` filename header.
- Diff highlighting (`+`/`-` lines).

The line-annotation + title features land alongside the highlighter as
a small Rust transformer.

## Architecture

A single new transformer in `dmc-transform/src/builtin/pretty_code.rs`
that:

1. Walks the AST looking for `Node::CodeBlock`.
2. Reads `cb.lang` and `cb.meta` (which carries `title="..."` + `{1-3}`).
3. Tokenises `cb.value` via `syntect::easy::HighlightLines` (or
   `parsing::ParseState` + `highlighting::HighlightState` for finer
   control).
4. Emits an HTML string containing `<pre data-theme="..." data-lang="..."><code><span style="color:..."> ... </span> ...</code></pre>`.
5. Wraps the original `CodeBlock` in a `JsxElement` that owns the
   structured spans (plays nicer with downstream MDX-body emit; both
   emitters already render `JsxElement` correctly, no emitter changes
   needed).

```
   Before                  After
   ------                  -----
   CodeBlock {             JsxElement {
     lang: "ts",             name: "pre",
     meta: "{2,4}",          attrs: [data-theme, data-lang, data-meta],
     value: "..."            children: [
   }                           JsxElement {
                                 name: "code",
                                 children: [
                                   span+style for each token,
                                   ...
                                 ],
                               }
                             ],
                           }
```

## Step-by-step

Each step compiles + tests pass before the next.

### Step 1 -- workspace dep + asset bundle

- Add to workspace `Cargo.toml`:
  ```toml
  syntect = { version = "5", default-features = false, features = ["default-fancy"] }
  ```
  `default-fancy` uses `fancy-regex` (pure Rust). Drop to `default-onig`
  if benchmarks show fancy-regex is too slow on TextMate's lookahead
  patterns; `onig` requires building C oniguruma (extra build dep).
- Wire `syntect.workspace = true` into `dmc-codegen/Cargo.toml`.
- Create `dmc-codegen/assets/themes/` and `dmc-codegen/assets/grammars/`.
  Bundle the popular VS Code themes:
  - `github-dark.tmTheme`
  - `github-light.tmTheme`
  - `dracula.tmTheme`
  - `one-dark-pro.tmTheme`
  - `vitesse-dark.tmTheme`
  Source: extract from VS Code extension `.vsix` files OR convert from
  `.json` themes via TmTheme-Editor / vscode-theme-converter. Document
  the source per file in `assets/themes/README.md`.
- Bundle grammars in `assets/grammars/` for the popular languages
  (~50): ts, tsx, js, jsx, rust, go, python, ruby, sh/bash, json, yaml,
  toml, html, css, scss, sql, md, mdx, dockerfile, etc. Source: VS Code
  language extensions ship them; copy `.tmLanguage.json` files.

### Step 2 -- shared `SyntaxBundle` global

- File: `dmc-codegen/src/highlight.rs` (new).
- One-time setup: load themes + grammars at first use, cache forever.
  ```rust
  pub struct SyntaxBundle {
    pub syntaxes: SyntaxSet,
    pub themes: ThemeSet,
  }

  pub fn bundle() -> &'static SyntaxBundle {
    static B: OnceLock<SyntaxBundle> = OnceLock::new();
    B.get_or_init(|| {
      let syntaxes = SyntaxSet::load_from_folder(
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/grammars")
      ).expect("load syntect grammars");
      let themes = ThemeSet::load_from_folder(
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/themes")
      ).expect("load syntect themes");
      SyntaxBundle { syntaxes, themes }
    })
  }
  ```
- Themes and grammars are heavy to parse (~10-50 ms total). Doing this
  once per process amortises across every code block in every build.

### Step 3 -- highlighter wrapper

- File: `dmc-codegen/src/highlight.rs`.
- Public fn:
  ```rust
  pub fn highlight_code(
    code: &str,
    lang: Option<&str>,
    theme_name: &str,
  ) -> Vec<Vec<(syntect::highlighting::Style, &str)>>;
  ```
- Looks up grammar by extension or scope name; falls back to
  `find_syntax_plain_text` when language unknown (so build never errors
  on niche langs -- output is plain `<pre><code>...</code></pre>`).
- Uses `LinesWithEndings::from(code)` + `HighlightLines::highlight_line`
  to produce per-line, per-token `(Style, &str)` pairs.

### Step 4 -- `dmc-transform/src/builtin/pretty_code.rs`

- New file. Pub struct `PrettyCode { theme: String }`.
- `impl Transformer for PrettyCode`:
  - `name() -> "pretty-code"`
  - `transform(&self, doc, meta, engine)`:
    - Walks via existing `walk_root` + a `Visitor` matching `Node::CodeBlock`.
    - For each match: invokes `render_code_block(&cb, &meta, &theme)`
      -> `JsxElement`.
    - Returns `NodeAction::Replace(vec![Node::JsxElement(rendered)])`.
- Private helpers:
  - `parse_meta(&str) -> CodeMeta { title: Option<String>, line_marks: Vec<LineMark> }`
    -- splits `title="x" {1,3-5}` into a typed struct.
  - `render_code_block(cb: &CodeBlock, meta: &CodeMeta, theme: &Theme) -> JsxElement`
    -- runs `highlight_code`, emits per-token `<span>` JsxElements with
    inline `style="color:#xxxxxx"` attrs.
  - `apply_line_marks(spans: &mut Vec<Span>, marks: &[LineMark])`
    -- wraps individual line spans with `data-highlighted-line` etc.

### Step 5 -- feature flag in `dmc-core/Cargo.toml`

- `pretty-code` feature already exists in `dmc-core/Cargo.toml`. Wire
  it through to `dmc-transform`:
  ```toml
  dmc-transform = { path = "../dmc-transform", features = ["pretty-code"] }
  ```
- `dmc-transform/Cargo.toml`: add `pretty-code = ["syntect"]` (or
  inline-default).

### Step 6 -- pipeline registration

- File: `dmc-transform/src/pipeline.rs::Pipeline::with_defaults`.
- Append `PrettyCode::new(theme)` to the default chain when
  `pretty-code` feature is on.
- New `cfg.compile.pretty_code_theme: Option<String>` (default
  `"github-dark"`) plumbs the theme name through `EngineConfig` ->
  `CompileConfig` -> the transformer.
- New `cfg.compile.use_native_pretty_code: bool` (default `true`).
  When `false`, skip the native transformer; user falls back to sidecar
  `rehype-pretty-code`.

### Step 7 -- sidecar gate update

- File: `dmc-core/src/engine/compile.rs::CompileConfig::has_js_plugins`.
- When native pretty-code is enabled, **strip** `rehype-pretty-code`
  (and `shiki`) from the rehype plugin lists before checking. So a
  user whose only configured rehype plugin is `rehype-pretty-code` no
  longer triggers the sidecar at all -> falls back to the native fast
  path.

### Step 8 -- snapshot tests

- `dmc-transform/tests/pretty_code.rs`: assert specific code blocks
  render to expected JSX shape.
- `dmc-codegen/tests/pretty_code_html.rs`: assert downstream HTML
  output matches `rehype-pretty-code`'s for a representative code
  block. Use a snapshot file in `tests/fixtures/`.

### Step 9 -- bench delta

- Run `cargo run --release --example bench`.
- Update `docs/sidecar-path-perf.md` reference table with new numbers.
- Acceptance: sidecar+kitchen-sink @N=1000 <= 800 ms.

## Things that will surprise you

- **Theme parse cost is real.** `.tmTheme` files are XML/JSON; parsing
  them takes 5-15 ms each. With ~5 themes that's 25-75 ms of one-time
  startup. Hide it behind `OnceLock` per the bundle setup.
- **Grammar parse cost is bigger.** `.sublime-syntax` /
  `.tmLanguage.json` files can be 50-200 KB each. ~50 grammars = ~5 MB
  to parse. Use `SyntaxSet::load_from_folder` with the `dump` feature
  to serialise grammars to a binary blob (`.packdump`); load that
  instead at runtime (~3-5x faster startup).
- **Per-token allocation.** Each highlighted token becomes a `JsxElement`
  with attrs. For a 100-line file with ~500 tokens, that's 500
  allocations. Likely fine; profile if it matters.
- **Theme color format.** `syntect::highlighting::Style.foreground` is
  `Color { r, g, b, a }`. Convert to `#rrggbb` when emitting the
  inline `style=` attr. Drop `a` (HTML inline doesn't need alpha for
  opaque text).
- **Output style:** `rehype-pretty-code` uses CSS variables
  `--shiki-light` / `--shiki-dark` for dual-theme output. Not
  supported in v1. Single-theme only. If users want dual-theme, they
  opt out via `use_native_pretty_code = false`.
- **Unknown languages:** `find_syntax_by_token` returns `None` ->
  fall back to `find_syntax_plain_text` -> output is `<pre><code>...</code></pre>`
  with no highlighting. Don't error.
- **Line annotation parsing:** meta is space-separated
  `{1,3-5} title="x"`. Tolerant parser; ignore unknown directives.
- **Co-existence with `rehype-pretty-code`:** if user explicitly listed
  `rehype-pretty-code` in their plugin config AND `use_native_pretty_code`
  is true, native wins -> log a one-time diag explaining the skip.
  Don't run both.
- **Bundle size:** ~5 MB of themes + grammars baked into the binary via
  `include_bytes!` (or `include_dir!` macro). Gives a self-contained
  binary. Alternative: load from disk at runtime (`load_from_folder`)
  -- requires the user to ship `assets/` alongside the binary, which
  is a worse UX. Bake them in.

## Out of scope (for this milestone)

- Custom user themes via `cfg.compile.pretty_code_theme = "path/to/theme.tmTheme"`. Deferred.
- Loading additional grammars at runtime. Deferred.
- Diff syntax (`+`/`-` line markers). Deferred to a follow-up unit.
- Inline code highlighting (`` ` `` spans with language). Deferred.
- Dual-theme output (`--shiki-light` / `--shiki-dark`). Deferred.

## Why not WASM-compiled shiki?

Possible but adds a 5-10 MB WASM blob to the binary, plus wasmtime
runtime cost. `syntect` is pure Rust, ~3-5 MB binary impact, ~native
parsing speed. No competition.

## Why not tree-sitter?

Tree-sitter is the right choice for editor highlighting (real syntax
trees, incremental parsing, semantic queries). But:
- Themes don't translate one-to-one with VS Code themes; users would
  see different colours than they expect.
- Less coverage than textmate grammars for niche languages.
- Per-language opinionated highlighter -- no shiki theme parity.
- Bigger binary (each language is a compiled C grammar).

For dmc's use case (highlighting code blocks in mdx so the rendered
output matches what users see in VS Code), textmate via syntect is the
correct tool. Tree-sitter would be a separate follow-up if anyone asks.
