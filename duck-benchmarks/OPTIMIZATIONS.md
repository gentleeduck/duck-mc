# OPTIMIZATIONS - open perf work and timeline debt

We are not optimizing further right now. The correctness/perf problem is
solved: the native path runs ~30-130x faster than velite at 1000 files
(see `README.md` - 132x vs `velite+gfm`, ~31x vs `velite+kitchen-sink`,
phase 5). This file is the map for anyone who wants to push further; the
maintainer can point you in the right direction.

Everything below is grounded in the code as it stands. Estimates are
honest ranges with the reasoning attached; where a number needs a bench
to confirm, it says so. "Cheap" vs "invasive" is called out per item.

One number to keep in front of you the whole time, from
`phase-7-g-hardening/flamegraph/stage-profile.txt` (single realistic
fixture, release):

```
lex        4.63 us/iter   1.0%
parse      7.48 us/iter   1.6%
transform  370.5 us/iter 77.4%   <- of which pretty_code (syntect) is 97%
codegen    95.95 us/iter 20.0%
```

So: `pretty_code` (syntect highlight) is ~75% of total per-file work.
Lex+parse together are ~2.6%. Codegen is ~20%. Plan your effort
accordingly - shaving the lexer's `Vec<Token>` alloc is real but it is
single-digit-percent-of-2.6%, i.e. invisible end to end. The lever that
moves the headline is syntect.

---

## dmc-lexer

### Token vector allocation

`Lexer::new` already pre-sizes: `tokens: Vec::with_capacity(source.len() / 8)`
(`dmc-lexer/src/lib.rs:43`). That heuristic is decent for prose-heavy
MDX; it under-shoots for token-dense JSX/markup (lots of short tokens)
and over-shoots for big fenced code blocks (one `CodeFenceContent` token
per line of code, not per byte). Options, in rough order of payoff:

- Tune the divisor or make it content-aware (e.g. divide by 6 for files
  with a high `<` density). Payoff: removes 0-2 `Vec` regrowths per
  file. End-to-end: noise (lex is 1% of total).
- `SmallVec<[Token; N]>` for the token list. `Token` is large (see
  below), so a stack-inlined `N` of any useful size blows the stack
  frame; not worth it. Skip.
- Streaming token API ("stream of tokens"): have the parser pull tokens
  on demand instead of materializing the whole `Vec` up front. This is
  the structurally interesting one but it is *invasive* - the parser
  does a lot of lookahead (`self.tokens.get(self.pos + 1)`, slice
  reslicing in `raw_source_for_token_range`, in-place token mutation on
  recovery paths - see `block/list.rs:518`, `block/blockquote.rs:352`).
  A pull-based lexer would need a bounded ring buffer and a rewrite of
  every `self.tokens[i] = ...` mutation site into something the buffer
  supports. Estimate: multi-day refactor, removes one `Vec<Token>`
  allocation + the peak-memory spike on huge files, but ~0% wall-clock
  on normal files. Only do this if memory on pathological inputs (a
  10 MB generated MDX) becomes a real complaint.

### Token size

`Token<'src>` (`dmc-lexer/src/token.rs:6`) is `{ kind: TokenKind, span:
Span, raw: &'src str }`. `TokenKind` is a `#[repr(u8)]`-ish enum but its
largest payloads are `(FenceChar, u8)` / `(EmphasisChar, u8)` so it is
~2 bytes. `Span` (`duck-diagnostic` `Span`, see
`diagnostic.rs:94`) is `{ file: Arc<str>, line: usize, column: usize,
length: usize }` = 16 + 8 + 8 + 8 = 40 bytes. `raw: &str` = 16 bytes.
So `Token` is ~64 bytes after alignment. Confirm with
`println!("{}", std::mem::size_of::<dmc_lexer::token::Token>())` before
acting.

What makes it big is the `Span`, specifically the three `usize` fields +
the fat `Arc<str>`. Options:

- Store byte offsets (`start: u32`, `len: u32`) on the token instead of
  `(line, column, length: usize)`, and compute line/column lazily from a
  newline index only when a diagnostic is actually emitted. That drops
  `Token` from ~64 to ~24 bytes (kind + 2x u32 + the `&str`, or 16 if
  you drop `raw` and reslice from offsets). On a 4000-token file that is
  ~256 KB -> ~64-96 KB of token buffer: a real 2-4x cut in lexer/parser
  working set, better cache behavior in the parser's lookahead. But it
  is *invasive*: `Span` comes from the external `duck-diagnostic` crate
  and is used directly all the way into the AST (`Node` variants all
  carry `Span`) and into `dmc-napi`'s `DiagnosticReport`. You would
  introduce an internal lightweight `ByteSpan` for tokens + AST and only
  lift to `duck_diagnostic::Span` at the diagnostic boundary. Estimate:
  2-3 day refactor touching lexer, parser AST, codegen span reads,
  napi. Payoff: measurable on big files, single-digit % on normal ones.
  Worth it if/when someone also does the streaming-lexer work, since
  both touch the same surface.
- Intern the path: see the `Span` `Arc<str>` note under dmc-parser - the
  `file` field is the same `Arc<str>` for every token in a file, so the
  16-byte fat pointer is pure overhead repeated N times. An interned
  `u32` file id would shave 12 bytes/token. Same crate-boundary problem.

Net: nothing in dmc-lexer moves the headline. Do the byte-offset rework
only as part of a broader span overhaul, and treat it as a
memory/cache-pressure win, not a wall-clock one. Needs a bench to put a
real number on it.

---

## dmc-parser

### `Text` nodes own `String`; no-op unescape/decode still allocate

`Text { value: String, span: Span }` (`ast/node.rs:151`). Every text run
in the document is a heap `String`. The values come through
`Parser::unescape_markdown` (`inline.rs:1352`) and `decode_entities_in`
(`inline.rs:259`), both of which early-return on the no-op case but the
early return is `return s.to_string();` (`inline.rs:261`,
`inline.rs:1354`) - so even the overwhelmingly common "no backslash, no
ampersand" path allocates a fresh `String`. (Same pattern in
`decode_entity` for the multi-char-entity branches, `inline.rs:306,312`,
but those are rare.)

Options:

- Make `unescape_markdown` / `decode_entities_in` return
  `Cow<'_, str>`: `Borrowed` on the no-op path, `Owned` only when work
  happened. *Cheap* change to the two functions themselves. But the
  caller (`inline.rs:366`, `:550`, `:849`, `:1029` etc.) currently
  shoves the result straight into `Text { value: String, ... }` /
  `Link { href: String, ... }` - so to actually skip the allocation the
  AST field has to become `Cow<'src, str>`, which means threading a
  `'src` lifetime through `Node`. That is *invasive*: `Node` is
  `#[derive(Serialize, Deserialize)]` (used by the blake3 file cache and
  by `dmc-napi`), every AST struct grows a lifetime, every transformer
  signature (`fn transform(&self, doc: &mut Document<'src>, ...)`)
  changes, and transformers that *create* `Text` from owned data
  (`pretty_code`, `emoji`, autolink) need `Cow::Owned`. Estimate: large,
  1-2 weeks with the serde fallout. Payoff: removes ~1 alloc per text
  run - on a prose page that is dozens to low hundreds of allocs/file -
  but per the stage profile, parse is 1.6% of total, so the end-to-end
  win is well under 1%. Not worth it on perf grounds alone. The reason
  to do it would be peak memory on huge files, and even then a cheaper
  half-measure is below.
- Cheaper half-measure: keep `value: String` but make the no-op path
  reuse the input when the caller already owns a `String` it is about to
  drop, or at minimum add a `&'src str -> &'src str` fast-path so the
  caller can `Text { value: borrowed.to_string() }` only once instead of
  twice (today `decode_entities_in(&unescape_markdown(&href))` allocates
  twice even when neither does anything - `inline.rs:849`). Combine the
  two passes into one scanner that handles both `\` and `&` in a single
  walk and returns `Cow`. *Cheap-ish*, removes the double allocation on
  link/image hrefs and titles. Payoff: small but free.

### `Span` carries `Arc<str>` -> clone is an atomic refcount bump

Every `Span::clone()` (and there are many - `block/mod.rs`, `inline.rs`,
list/blockquote recovery, the emphasis resolver clones `span` per
produced node, `inline.rs:56,106`) bumps an atomic refcount on the path
`Arc<str>`. Atomics are not free, and on a parallel build (rayon, one
file per worker) the path arc for a given file is only ever touched by
one thread - so the atomicity is pure waste, it just is not contended.

Options:

- Intern paths into a process-wide `Vec<Arc<str>>` and store a `u32`
  file id on the span instead of the `Arc<str>`. `Span::clone()` becomes
  a `u32` copy. Resolve the id to a path string only when rendering a
  diagnostic. Also shrinks `Span` 16 -> 4 bytes, which shrinks `Token`
  and every AST node. *Invasive* for the same reason as the byte-offset
  rework: `Span` is the external crate's type. You would carry an
  internal span type and convert at the boundary. Estimate: rolled into
  the same span-overhaul project as above; ~2-3 days. Payoff: removes
  one atomic op per span clone (thousands per file) - but again parse is
  ~1.6%, so wall-clock impact is sub-1%. Real value is the size cut.
- Cheaper: `Rc<str>` instead of `Arc<str>` is not an option because the
  AST crosses thread boundaries during the rayon build (the `Document`
  is built on the worker thread and consumed there, so it *could* be
  `Rc`, but `Pipeline` is `Send + Sync` and the types are shared)... not
  worth unpicking. Skip.

### `Vec<Node>` AST + drain/insert churn in the emphasis resolver

`resolve_emphasis_delims` (`inline.rs:21`) does `out.drain(lo..hi)`
(`:69`), `out.remove(lo)` (`:83`), `out.insert(open_out_idx, ...)`
(`:106`), and then walks the whole `delims` slice fixing up `out_idx`
offsets (`:112`) - all O(n) shifts inside an O(delims^2) outer loop. On
text with a lot of emphasis markers (or pathological `***`-soup) this is
quadratic-ish. On normal prose it is fine - a paragraph has a handful of
emphasis runs.

Options:

- `SmallVec<[Node; N]>` for the hottest inline child vectors
  (paragraph/emphasis/link children). Most inline child lists are short
  (a `<strong>` wraps 1-3 nodes). Inlining the first ~4 avoids the heap
  alloc for them. *Cheap-ish* but `Node` is large (it is the big enum +
  `Span`s), so even `N=2` makes the parent struct fat; measure the
  tradeoff. Payoff: removes a `Vec` allocation per short inline wrapper -
  dozens per page - but parse is 1.6% of total. Needs a bench; expect it
  to not show up end to end.
- Rework the resolver to build a result `Vec` in one pass instead of
  mutating `out` in place with shifts, or use an index-based linked
  structure for the delimiter stack so removing interior delimiters does
  not require renumbering. *Moderate* effort, contained to `inline.rs`.
  Payoff: turns the pathological case from quadratic to ~linear; on
  normal input, nothing. Worth it only if a fuzz/DoS input shows up
  here (the existing G2 work already bounded link-label recursion and
  killed two parser DoS inputs - emphasis-resolver blowup has not been
  reported, but it is the obvious next candidate).

### In-place token mutation on recovery paths

`try_promote_text_blockquote_marker` / `try_promote_text_list_marker`
(`block/mod.rs:376`, `block/blockquote.rs:352`) and the list-recovery
code at `block/list.rs:505-545` mutate `self.tokens[pos].kind`,
`.raw`, and `self.tokens.insert(pos+1, text_tok)` to "re-lex" a token
the lexer got wrong in context. The `Vec::insert` is O(n) shift of the
rest of the token buffer; it only fires on the relatively rare
list/blockquote-after-marker recovery path, so it is not a hot path -
but it is the thing that makes a streaming lexer hard (you cannot
`insert` into a stream you have already consumed). It is also a
correctness smell: the parser is patching the lexer's output rather than
the lexer producing the right token. Right fix: give the lexer enough
context to emit `BlockQuoteMarker` / `UnorderedListMarker` correctly
inside list items, or model "this token can be reinterpreted" as a token
flag instead of a buffer edit. *Moderate* effort, mostly in dmc-lexer's
`dispatch.rs`. Payoff: not perf (cold path) - it is the precondition for
the streaming-lexer item and it removes a fragile mutation pattern.

### Unicode-punctuation classification (G5.3)

`is_unicode_punct` (`inline.rs:162`) is called per inline punctuation
char during flanking-rule checks (`inline.rs:611,616`). It already has
an ASCII fast path - but only for ASCII *punctuation*:
`if c.is_ascii_punctuation() { return true; }`. For an ASCII *letter or
digit* (the common case at a delimiter boundary - `*word*`), it falls
through to `unicode_general_category::get_general_category(c)`, a
multi-level table lookup, just to learn "not punctuation". Fix: add
`if c.is_ascii() { return c.is_ascii_punctuation(); }` *above* the
general-category call. *Trivial* (one line), risk-free, and it short-
circuits the common boundary check. Payoff: small - this is on the
emphasis path which is a slice of parse's 1.6% - but it is free, so do
it. Needs a parser micro-bench re-run (`cargo bench -p dmc-parser
--bench parse`) to confirm no regression and record any improvement in
`BENCHMARKS.md`.

---

## dmc-transform

Per the stage profile, `transform` is 77% of per-file work and
`pretty_code` is 97% of that. Everything else in the pipeline
(`assign_heading_ids` 0.2%, `code_import` 0.2%, `bare_url` 1.8%,
`autolink_headings` 0.7%) is noise. So: optimizing any non-`pretty_code`
transformer is wasted effort until syntect is faster. Concretely:

### pretty_code / syntect (the only lever that matters)

`PrettyCode::transform` walks code blocks and calls
`dmc_highlight::highlight_code_multi` (`builtin/pretty_code.rs`,
`dmc-highlight/src/lib.rs:193`). syntect's cost per block is
`O(parse) + O(themes * scope_walk)`; the multi-theme path already shares
the grammar parse across themes (good - `lib.rs:155-243` comment). Open
work:

- **Cache highlighted output by `(code, lang, theme-list)` hash.** Docs
  sites repeat snippets (the same `bun add ...` line, the same import
  block) across many pages, and a watch-mode rebuild re-highlights
  unchanged blocks. A process-wide `HashMap<(u64 hash), Rc<rendered>>`
  in front of `highlight_code_multi`, or - better - lift it into the
  existing blake3 file cache so it survives across builds. The file
  cache already keys per-file output (`engine/cache.rs`), so an
  unchanged file already skips highlight entirely; the gap is *unchanged
  blocks in changed files* and *identical blocks across files*. Payoff:
  on a real docs build with duplicated code blocks, plausibly 10-40% off
  total build time; on a corpus with all-unique blocks, zero. *Moderate*
  effort (hashing + a `Mutex<HashMap>` or `dashmap`), uncertain payoff -
  needs a bench against `apps/duck`'s real corpus.
- **Skip highlight when the language is unknown.** Today an unknown lang
  falls back to the plain-text grammar (`lib.rs:200`,
  `find_syntax_plain_text`) and still runs the full
  `ParseState`/`HighlightState` machinery to produce one un-styled token
  per line. For a code block with no info string (or `lang="text"`), you
  can short-circuit: emit the lines as plain text spans without invoking
  syntect at all. *Cheap*, contained to `pretty_code` / `dmc-highlight`.
  Payoff: depends entirely on how many `text`/no-lang blocks the corpus
  has; on a config-heavy docs site (lots of bare ``` blocks) it could be
  meaningful. Needs a bench.
- **Lazy syntax-set loading.** `SyntaxBundle::get` (`lib.rs:45`) parses
  *every* bundled grammar on first use (~25-100 ms one-time, per its own
  doc comment). A docs build only uses a handful of languages. Building
  a `SyntaxSet` lazily / on-demand per language, or shipping a
  pre-serialized `.packdump` (`syntect`'s `dump_to_file` /
  `from_dump_file`) instead of parsing `.sublime-syntax` YAML at
  startup, would cut that 25-100 ms to a few ms. Payoff: pure startup
  win - matters for the per-file `dmc compile` CLI and tests, invisible
  on a 1000-file build where it amortizes. *Moderate* effort (build.rs
  change to emit a dump). Worth it for CLI latency.
- syntect itself: there is no easy 2x here. `two-face`/newer syntect
  versions, or a different highlighter (tree-sitter-based) is a project,
  not an optimization. Out of scope.

### Single-walk pipeline (already done)

Phase 2 merged lex+parse into the compile path and codegen does a single
DFS fanning to both sinks (`dmc-codegen/src/lib.rs` `Walker`,
`NodeSink`). What is left on the pipeline structure itself: each
transformer does its own tree walk (`walk_root` / `Visitor` in
`dmc-transform/src/visit.rs`), so a file with N active transformers gets
N AST traversals. They are cheap traversals (the expensive transformer,
`pretty_code`, only touches code-block nodes), so fusing them into one
visitor pass is a *moderate* refactor for ~nothing measurable. Skip
unless the transformer count grows a lot.

---

## dmc-codegen

### Document rendered twice (HTML + MDX body)

The walker fans every node to both `HtmlEmitter` and `MdxBodyEmitter` in
one DFS (`Walker::walk`, `lib.rs:64`) - so the *traversal* is shared, but
each emitter builds its own output `String` independently and the work
inside each `enter`/`leave` is duplicated logic. That is intentional
(the two outputs are genuinely different), but: a consumer that only
needs HTML (sidecar disabled, `include_html` true, no `module` output)
still pays the MDX-body walk. `CompileConfig::for_render` already turns
off `emit_html` when sidecar will run (`engine/compile.rs:154`), but
there is no symmetric "skip MDX body when only HTML is wanted" -
`finalize` always runs `MdxBodyEmitter` (`engine/compile.rs:289+`).
Adding an `emit_body` short-circuit there is *cheap*. Payoff: ~half of
codegen's 20% on HTML-only consumers -> ~10% off total for them; 0% for
the common case that wants both. Worth doing.

### String building

Both emitters start with `String::new()` (`html.rs:108`, `mdx.rs:127`) -
no pre-size - and grow by `push_str`, with a fair amount of
`push_str(&format!(...))` (`html.rs:173,174,194`, `mdx.rs:163,213,283`)
which allocates a throwaway `String` per call just to append it. Fixes,
all *cheap*:

- Pre-size the output buffer from a heuristic (HTML output is roughly
  `1-2x` source length; you have the source). Removes the early
  doubling reallocs - a handful per file.
- Replace `push_str(&format!(...))` with `write!(self.out, ...)` (it
  implements `fmt::Write`) - no intermediate `String`. Removes one alloc
  per such call site.
- Reuse emitters across files via an object pool: `HtmlEmitter` /
  `MdxBodyEmitter` each carry a `DiagnosticEngine` and the `out` buffer;
  on a 1000-file build that is 2000 emitter constructions + 2000 buffer
  allocations. A `thread_local!` pool that `out.clear()`s and reuses the
  buffer would remove all of them. *Moderate* effort (need to reset all
  emitter state cleanly, or it leaks output between files - a bug
  magnet). Payoff: codegen is 20% of per-file work, allocation is a
  slice of that; expect low-single-digit % end to end. Bench it.

Net for dmc-codegen: the `emit_body` skip is the only one with a
double-digit-% story, and only for HTML-only consumers. The rest is
free-ish cleanup.

---

## dmc-core

### Per-file setup cost

`Compiler::compile_with_pipeline` (`engine/compile.rs:257`) builds a
fresh `Pipeline` *per file* via `Pipeline::with_defaults_for`
(`compile.rs:281`), inside the rayon `par_iter().map()` in
`engine/collection.rs:86`. That allocates ~10 `Box<dyn Transformer>` per
file plus the small per-transformer structs (`PrettyCode` clones a few
`String`s for theme names + shape options). The expensive bit -
`SyntaxBundle::get()` (the syntect grammar/theme parse) - is *not*
per-file; it is a process-wide `OnceLock` (`dmc-highlight/src/lib.rs:45`)
that the first file pays and the rest get free. So this is ~10
small heap allocs per file: real, but parse+transform-setup is a
sliver. Fix: build the `Pipeline` once per build (or once per rayon
worker via `thread_local!`) and pass `&Pipeline` into the per-file
compile. `Pipeline` is already `Send + Sync` by design (`pipeline.rs`
doc comment says exactly this), so a shared `&Pipeline` across all
workers is fine. *Cheap-to-moderate* (thread the reference through
`compile_with_pipeline`'s signature). Payoff: removes ~10 allocs/file;
expect sub-1% end to end. Worth it as cleanup; do not expect a headline
move.

### blake3 file cache (already done, two-layer)

Phase 6 added the persistent per-file cache (`engine/cache.rs`): one
JSON record per file keyed by `(dmc_version, source_bytes, path,
cfg_fingerprint)`, stored at `<output_dir>/.cache/dmc/{16-hex}.json`.
Plus the math cache (`<output_dir>/.cache/math.json`,
`engine/mod.rs:54`). Open work:

- The cache only stores *clean* runs (`collection.rs:122` - `if !dirty`)
  so a file with a warning re-compiles every build. That is deliberate
  (you want to keep seeing the diagnostic) but it means a docs repo with
  even one persistent warning never gets a fully warm cache. A cache-the-
  diagnostics-too design would fix it; *moderate* effort, mild payoff.
- One file per record means a cold build with a warm cache does N small
  `fs::read` + `serde_json::from_str` calls. A single packed cache file
  (or sled/redb) would amortize the syscalls. *Moderate*; payoff only on
  huge corpora.
- See the pretty_code item above: caching highlighted *blocks* (not just
  whole files) inside this layer is the high-value extension.

### rayon parallelism granularity

`collection.par_iter()` over file paths (`collection.rs:58`) - one task
per file, default rayon work-stealing. For a corpus of a few hundred
files of wildly varying size (one giant API-reference page next to many
tiny ones), the giant file's worker becomes the long pole. rayon's
default chunking is fine here; the only lever would be sorting paths
biggest-first so the long pole starts early (LPT scheduling). *Cheap*
(sort by `fs::metadata` len before `par_iter`). Payoff: shaves the tail
on imbalanced corpora - low single-digit % of wall clock at best, zero
on balanced ones. Bench against `apps/duck` before bothering.

---

## dmc-napi

### `BuildReport` re-reads collection files just to count records

`build` (`dmc-napi/src/lib.rs:158`) calls `Engine::run`, which writes one
`<output_dir>/<name>.json` per collection - then immediately
`std::fs::read_to_string` + `serde_json::from_str` each of those files
*again* just to fill `BuildCollectionReport.records` with a count
(`lib.rs:232-247`). On a big collection that JSON file is megabytes; you
parse it twice (once to write, once to count). Fix: have `Engine::run`
return the record counts (it has them in hand - `collection.rs` collects
the `Vec<Value>` before writing). *Cheap-to-moderate* (thread a return
value out of `Engine::run`). Payoff: removes one large-JSON parse per
collection per build - on a docs site that is one ~MB parse, tens of ms.
Worth it.

### `serde_json::to_value` double conversion

`compile` / `compile_many` (`lib.rs:15,35`) do
`serde_json::to_value(&out)` to produce a `serde_json::Value`, which
napi-rs then walks again to build the JS object. That is AST -> serde
`Value` tree (alloc-heavy) -> JS object. For the single-file `compile`
entry points (used by tooling, not the bulk build) this is two full
traversals + an intermediate tree. A direct `CompileOutput -> JsObject`
via `napi`'s `ToNapiValue` (or returning a pre-serialized JSON *string*
and letting JS `JSON.parse` it, which is often faster across the FFI
than walking a `Value`) avoids the middle tree. *Moderate* effort
(implement the conversion, or restructure to ship a string). Payoff:
only matters for callers hammering `compile()` per file; the bulk
`build()` path goes through the engine, not these. Bench before
investing.

### General FFI cost

The `build()` boundary crosses once per build (not per file - the engine
loop is entirely Rust-side), so the only per-build FFI payload is the
`BuildReport`: `Vec<DiagnosticReport>` (one per diagnostic - usually
few) + `Vec<BuildCollectionReport>` (one per collection - few) + the
empty `errors`. That is tiny. No work needed here beyond the two items
above. The diagnostics list does `d.message.clone()` /
`d.help.clone()` / `d.code.code().to_string()` per diagnostic
(`lib.rs:250-264`) - trivial unless a build produces thousands of
diagnostics, in which case the user has bigger problems.

---

## dmc-sidecar (JS)

Since phase 5, the Node sidecar (`dmc-sidecar/index.mjs`) only runs when
the user configures a *foreign* remark/rehype plugin that the native
pipeline does not own (`engine/compile.rs:140` `filter_native_owned_*`,
`has_js_plugins`). With a stock config it is never spawned - the
"no-op forwarder" path is just: native compile produces HTML, no JS
plugins configured, return. So there is nothing to optimize for the
common case; it is already gated to zero cost.

When it *is* used: the sidecar already caches the built `unified`
processor by `JSON.stringify([remarkSpecs, rehypeSpecs])`
(`index.mjs:42-57`), so it builds the plugin chain once per
plugin-config, not once per file. The remaining cost is one Node process
spawn per build (cold ~50-150 ms) + one line of JSON over stdin/stdout
per file + `unified`'s own parse-rehype-stringify per file (which
re-parses the HTML the native side already produced - `remarkParse` then
`remarkRehype` then `rehypeRaw`). That double-parse is inherent to
"hand HTML to a markdown plugin chain"; the only way out is not using
foreign plugins. Not worth engineering around - it is opt-in and the
user accepts the cost by configuring a foreign plugin.

---

# Timeline debt - things done quick rather than right

These are visible in the code or git history as expedient choices, not
the correct ones. None are bugs in shipped behavior; they are
maintenance/cleanup debt.

### `Origin::Inline(rel.to_string().leak())` - deliberate memory leak in an example

`dmc-core/examples/flamegraph_consumer.rs:81`:

```rust
let meta = Arc::new(SourceMeta { path: Arc::from(rel), origin: Origin::Inline(rel.to_string().leak()) });
```

`Origin::Inline` wants a `&'static str` (`dmc-diagnostic/src/metadata.rs:24`).
Rather than use a fitting variant, the example `.leak()`s a `String` per
file per pass - and `flamegraph_consumer` loops the whole ~370-file
corpus repeatedly for ~5 s, so it leaks hundreds of small strings per
run. Harmless in a short-lived profiling binary, but it is the wrong
construct. Right fix: use `Origin::File(PathBuf::from(rel))` (the bytes
*do* come from a file) or `Origin::Memory`. One-line change, zero risk.
Effort: trivial.

### `flamegraph.rs` / `flamegraph_consumer.rs` hard-code the output phase folder

Both examples write directly into
`duck-benchmarks/phase-7-g-hardening/flamegraph/` (the path is baked into
`main` - `flamegraph.rs:90`, `flamegraph_consumer.rs:95`). `GUIDE.md`
section 3 even documents the workaround: "update the hard-coded path in
those two examples when you create the new folder." So cutting a new
phase requires editing source. Right fix: take the output dir as an arg
(`std::env::args()`) with the current path as the default, or read a
`DMC_BENCH_PHASE` env var. `profile.rs` already takes its iteration count
as `args().nth(1)` so the pattern exists. Effort: ~15 min each.

### Early-return-then-allocate in `unescape_markdown` / `decode_entities_in`

Covered under dmc-parser above. `return s.to_string();` on the no-op path
(`inline.rs:261,1354`) allocates when it could return a borrow. The
quick version was "function returns `String`, so the no-op also returns
`String`"; the right version is `Cow<'_, str>` (cheap for the functions,
invasive for the callers because the AST field is `String`). It is debt
because the cheap-but-half fix (return `Cow`, callers still
`.into_owned()` at the `Text` boundary, but at least the *double*
allocation on `decode_entities_in(&unescape_markdown(&x))` collapses to
one) was never taken.

### Recovery paths mutate the token buffer in place

`block/list.rs:505-545`, `block/mod.rs:376`, `block/blockquote.rs:352`
patch `self.tokens[i].kind` / `.raw` and `Vec::insert` synthetic tokens
to fix the lexer's context-blind output. Covered under dmc-parser. It is
debt because the right fix - give the lexer the context, or model
reinterpretable tokens explicitly - is more work than patching after the
fact, so the patch shipped. It also blocks the streaming-lexer option.

### `serde_yaml` (deprecated, unmaintained) still used in dmc-core

`dmc-core/src/loaders.rs:59` (`YamlLoader`) and
`dmc-core/src/engine/accumulator.rs:26` (frontmatter parse) use
`serde_yaml = "0.9"` (`Cargo.toml:33`, `dmc-core/Cargo.toml:42`).
`serde_yaml` was officially deprecated by its author (dtolnay) in 2024;
it still works but gets no fixes. Right fix: migrate to a maintained
fork - `serde_norway` (drop-in `serde_yaml` API fork) is the lowest-
effort swap; `serde_yml` is another option. Effort: small (mostly a
`Cargo.toml` change + import rename + a test pass). It is debt because
the migration is "do it later" and later has not come; `cargo deny`
(`deny.toml` exists in the repo) will eventually flag the advisory.

### Two JS lockfiles checked in

`package-lock.json` (npm) and `pnpm-lock.yaml` (pnpm) both live at the
repo root, with `pnpm-workspace.yaml` present - so pnpm is the intended
package manager and `package-lock.json` is stale cruft from an earlier
npm-based setup (it is tiny - 1.8 KB - because the JS surface is just
`plist` + dev deps). Two lockfiles drift independently and confuse CI /
contributors about which PM to use. (Note: the task brief mentioned a
`bun.lock` too - there is no `bun.lock` in the tree, just these two.)
Right fix: delete `package-lock.json`, keep `pnpm-lock.yaml`, and add a
`"packageManager": "pnpm@..."` field to `package.json` so tooling picks
the right one. Effort: trivial.

### `unwrap()` / `expect()` on IO in the example binaries

The bench/flamegraph/profile examples are studded with `.unwrap()` /
`.expect()` on filesystem calls: `fs::create_dir_all(&out_dir).unwrap()`
(`flamegraph_consumer.rs:99`), `fs::read_dir(&d).unwrap_or_else(|e|
panic!(...))` (`:60`), `fs::File::create(&svg_path).expect("open svg")`
(`:147`), `report.flamegraph(f).expect("write svg")`, same shape in
`flamegraph.rs` and `bench.rs`. For throwaway dev tooling that is
*acceptable* - a panic with a clear message is fine when a developer ran
the wrong command - but it is the kind of thing that gets copy-pasted
into non-example code. It is debt only in the sense that "examples are
the template people copy"; low priority. Right fix if anyone cares: have
`main` return `Result<(), Box<dyn Error>>` and `?` the IO. Effort: small
per file, low value.

### Per-file `Pipeline` construction

`compile.rs:281` rebuilds the whole transformer list per file inside the
rayon loop. Covered under dmc-core. Debt because `Pipeline` was designed
`Send + Sync` *specifically* so it could be shared (its own doc comment
says so) - and then the call site rebuilds it anyway. The "share one
`&Pipeline` across the build" wiring was never done.

### `Compiler::compile` has a leftover `// FIX:` marker

`engine/compile.rs:251-254`:

```rust
pub fn compile(source: &str, diag_engine: &mut DiagnosticEngine<Code>) -> CompileOutput {
  // FIX:
  Self::compile_with_pipeline(source, Path::new("."), &CompileConfig::new(), diag_engine)
}
```

The bare `// FIX:` is a parked TODO with no description - presumably "the
synthetic `Path::new(".")` means relative `file=...` directives and
`copy-linked-files` resolve against the cwd, not the real file." It is a
documented-as-such limitation of the convenience entry point (the doc
comment says "Use `compile_with_pipeline` for file-aware compilation"),
so it is fine, but the naked marker should either become a real comment
explaining the constraint or be removed. Effort: trivial.

---

# How to validate any of this

Read `duck-benchmarks/GUIDE.md` first - it covers quieting the machine,
which numbers mean something (velite is the control; mind the stddev
band), and how to record a phase folder.

The commands:

```sh
# headline: 10/100/1000-file size sweep, all six variants -> dmc-core/tmp/bench.json
cargo run --release -p dmc-core --features pretty-code --example bench

# stage split (lex / parse / transform / codegen + per-transformer) -> stdout
cargo run --release -p dmc-core --features pretty-code --example profile

# in-process flamegraph of the native path on one realistic fixture
cargo run --release -p dmc-core --features pretty-code --example flamegraph

# flamegraph over the real ~370-file apps/duck corpus
cargo run --release -p dmc-core --features pretty-code --example flamegraph_consumer

# parser micro-bench (criterion) - the regression gate; baselines in BENCHMARKS.md
cargo bench -p dmc-parser --bench parse

# pipeline-level compile bench
cargo bench -p dmc-core --bench compile
```

If you land a perf change: re-run `cargo bench -p dmc-parser --bench
parse`, update `duck-benchmarks/BENCHMARKS.md`, and if it plausibly
moves the compile pipeline, cut a new `phase-N-<label>/` folder per
`GUIDE.md` (and remember those two flamegraph examples hard-code the
phase path - see the timeline-debt section).
