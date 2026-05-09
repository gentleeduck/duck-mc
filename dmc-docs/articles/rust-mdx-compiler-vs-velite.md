# how i wrote a rust mdx compiler that beats velite by ~10x at p99

## the thesis

most of the perf posts you read measure the wrong thing, the median is
the cost when nothing is wrong, and that's not the cost users feel,
what users feel is the tail, the p99, the build that takes 3 minutes
and 48 seconds when you have a thousand mdx files, the wait between save and reload
when you're deep in a doc, that's the number that matters, and that's
the number i was going to fix.

this is how i did it.

## where the pain came from

i run [`@gentleduck`](https://gentleduck.org), it's an open source
org with a lot of packages and even more docs, thousands of mdx files
across a custom documentation framework i've been building for years,
we don't use `fumadoc` or any off-the-shelf docs site because we have
custom remark and rehype plugins on top, plus our own framework
underneath, switching to a docs framework would mean throwing away
years of work for no real gain, so the docs are mine and the
compiler that builds them is mine to fix.

the symptom was simple, edit one mdx file, wait three minutes and
forty-eight seconds, see the change, almost four minutes is not
"slow", almost four minutes is "different task", you've finished
thinking about the change before the loop comes back, by the time
the page reloads you've already moved on and now you're
context-switching to recover the thought, multiply 3:48 by every
member of the team and every ci build and the dev experience
problem is now an engineering tax.

i wrote down what i had to optimize for:

1. high throughput, thousands of files, growing
2. heavy plugins, gfm + math + mermaid + code highlighting + autolinks
   + custom transforms, all of them
3. p99, not p50

## why velite is slow (and it's not velite's fault)

velite is built on the unified.js stack, that means for every mdx
file the pipeline does this:

```
markdown text
  -> remark-parse        => mdast (markdown ast)
  -> remark-rehype       => hast (html ast)
  -> rehype plugins      => hast' (transformed)
  -> rehype-stringify    => html string
```

four ast representations per file, four pre-order traversals, each
pass allocates a new tree, walks every node, re-validates, then
throws the previous one on the gc heap, that's the structural cost,
and on top of that:

- `shiki` re-parses every code fence against its grammar+theme combo
  every call, no internal cache, the grammar load alone is tens of
  milliseconds and we run it per fence
- `rehype-katex` shells into the katex js engine for every `$...$`
  region, js engine startup + parse + render, per region

if your file has 10 code blocks and 3 math expressions, you pay 10
grammar loads and 3 katex startups, every build, no memoization
across files in the same session, so the cost grows linearly with
content density, and our docs are *very* dense.

this is the structural ceiling i was about to break.

## who i'm optimizing for

most perf posts brag about p50, the median build of a boilerplate
fixture, the hello-world demo where everyone wins, i don't care about
the median, when you have one mdx file every compiler is fast enough,
when you have ten even the slow ones feel quick, the pain only shows
up at the **tail**, the long files, the heavy collections, the
rebuild after one change to a shared snippet that gets imported into
200 docs, that's p99, that's the build server, the commit hook, the
hot loop, that's where you live as soon as you have real content.

so the audience for this work is:

- people with content sets large enough that build wall-clock matters
- people who feel the worst case, not the average
- people who want their tools to run in-process, not over ipc, on the
  hot path

if you have twelve files none of this applies, the median is fine for
you, use velite, this isn't for you.

## phase 1: the naive port

i started with the obvious shape, lex + parse + transform + emit,
four crates, four passes, same structure as the unified stack just in
rust, the bet was, if the structure is correct then the speedup comes
for free from moving to a fast language.

i built it, wrote a bench harness, ran the first numbers, here's what
came out at 1000 files, median wall time per build, lower is better:

- native dmc, plain markdown -> 10.65 ms
- sidecar + remark-gfm -> 404.86 ms
- sidecar + pretty-code -> 806.87 ms
- sidecar + kitchen-sink -> 2652.43 ms
- velite + remark-gfm -> 5984.50 ms
- velite + kitchen-sink -> 1375.06 ms

`sidecar+*` is the dmc native pipeline plus a node child running
whatever js plugins the user listed, `kitchen-sink` is the realistic
config with everything turned on, gfm + math + highlight + the works.

three things to read off this table, and most people would only read
one of them.

**the native column was already very fast.** 10ms for 1000 plain
markdown files, 100us per file, no obvious low-hanging fruit, easy
brag.

**the sidecar column was not.** 2.6 seconds for 1000 kitchen-sink
files, that's 30-50% of velite, better but only because rust was
doing some of the work, the bulk was still being paid to the node
sidecar running shiki and katex with no shared state.

**velite's number wasn't actually the worst.** look at
`velite+remark-gfm`, nearly 6 seconds, that's velite paying the node
overhead for plugins that are just structural transforms, the 4-pass
cost surfacing, no plugin work, just walking the tree four times.

if i'd stopped here and shipped the brag i'd be lying, the native
number is great but real users run with plugins, real users hit the
sidecar, real users run `sidecar+kitchen-sink`, that's the column i
had to move.

## how to read a bench

before phase 2 i went back to the harness and fixed three things i
had wrong, this part matters more than the optimizations, if you
can't read the bench you can't tell which optimization is real.

**measure what users feel, not what's clean to measure.** a single
build at p50 isn't the user's experience, the user's experience is
"edit, wait, see", wall clock from save to render, that includes the
file watcher debounce + the build itself + the framework's
incremental reload, i had to narrow on the build only, then put the
other costs back later as a separate column.

**variance > median.** at 1000 files the per-fixture medians were
tight but the size sweep had wide spread, p95 vs median was where the
story was, katex-heavy files in the 1000-file run were pulling the
mean up, some files took 3ms, some took 20ms, same config, same
content distribution, the slow ones were slow because the js engine
in the sidecar had a stop-the-world gc pause, that's a tail effect, a
median run hides it, so i started reporting p95 alongside median.

**don't confuse native with sidecar.** look at the table again,
`native` and `sidecar+kitchen-sink` are both dmc, the first runs the
in-process rust pipeline only, the second runs rust *plus* the node
sidecar with whatever plugins the user listed, real users configure
with plugins, the sidecar column is the user column, i had been
celebrating native and ignoring the column that mattered, fixed.

medians lie, p99 tells the truth, and the column you put in your
headline is the column you'll end up optimizing for, choose the right
column before you start.

## phase 2: stop walking the same tree

the four-pass model in js exists because each library wants its own
type, remark works on mdast, rehype works on hast, they're different
shapes, going between them means a translation pass.

i didn't need that, i had one type all the way through, there was no
reason to traverse it four times, so i rewrote the pipeline as a
single pre-order dfs, one walker, a trait called `NodeSink` with
`enter` and `leave` hooks, three sinks ride the same walk in parallel:

- `HtmlEmitter` writes the html string
- `MdxBodyEmitter` writes the jsx module source
- `Accumulator` collects frontmatter, imports, exports, table of
  contents, excerpt

all three update at every node, stack-balanced, no intermediate ast,
one walk does the work of four.

phase 2 numbers:

at 1000 files:

- native: 10.65 ms (phase 1) -> 11.92 ms (phase 2)
- sidecar + kitchen-sink: 2652.43 ms -> 2666.42 ms

read that again, the single-walk refactor moved the realistic
workload by ~0.5%, inside the noise band, two days of work, no win
on the bench.

this is the part that gets cut from brag posts, the
"architecturally correct" change didn't move the meter, but it wasn't
wasted, the single-walk model unblocked the next two phases, with one
walker and one tree shape i could attach native sinks for things
velite was paying js to do, i couldn't have done that with four-pass
plumbing, sometimes the refactor that doesn't move the bench unlocks
the one that will, that's how you do compiler work, you do the boring
structural part first because the interesting wins depend on it.

## phase 3: bring the highlighter in-process

shiki costs the most because it does the most, it loads grammars, walks
every code block, applies a theme, emits html, all in js, all across an
ipc boundary, with no shared cache.

rust has `syntect`, same job, native, in-process, with grammars and
themes baked in at compile time, no startup, no ipc, no per-call grammar
load, i bundled them into a new crate (`dmc-highlight`), wired it into
the transformer pipeline, ran the bench again.

at 1000 files:

- **native: 11.92 ms (phase 2) -> 47.42 ms (phase 3)**  <-- got slower
- sidecar + pretty-code: 854.87 ms -> 492.93 ms
- sidecar + kitchen-sink: 2666.42 ms -> 885.74 ms

look at native, it got **4x slower**, 12ms to 47ms.

a naive reading would call this a regression and revert, that reading
is wrong, the native column got slower because the bench fixture has
code blocks in it and now we're highlighting them in rust instead of
in the sidecar, the work moved, it didn't appear out of nowhere, that's
a load transfer, the sidecar column dropped because the highlighter is
no longer running there, net effect on the realistic workload, 2666ms
to 886ms, ~3x improvement on what the user actually pays.

before someone asks, dmc does *not* pay for transformers you don't
use, gating happens at three levels:

- **compile time**, every transformer is behind a cargo feature
  (`pretty-code`, `math`, `emoji`, `mermaid`, `npm-command`,
  `assets`), build with `--no-default-features` and the binary is
  ~4mb instead of ~12mb, the unused crates aren't even linked
- **config time**, `PipelineConfig` has `pretty_code: Option<...>`,
  `math_engine: Option<...>` and so on, set to `None` and
  `Pipeline::with_defaults_for(cfg)` doesn't push that transformer
  into the chain, it's not in the walk, period
- **per-file**, even when the transformer is in the pipeline, it only
  runs on nodes that match its pattern, no code block in the file,
  `PrettyCode` walks past zero matches and returns, no `$...$` in the
  file, `Math` does nothing, the only cost is one visit to each node,
  which is the cost you'd pay for the walker anyway

the SyntaxBundle (grammars + themes) loads once per process via
`OnceLock`, first call eats the load (~30ms cold), every call after
is free, the bench is over warm runs so this isn't in the numbers,
the 47ms in phase 3 is real per-file highlighting work on real code
blocks in the fixture.

so the 4x slower native column isn't unconditional cost, it's "here's
what it costs to highlight code blocks in rust when there are code
blocks", if your content is markdown without fences, the native
column stays where it was in phase 2.

this is the kind of trade you don't see if you only watch one number,
two numbers moving in opposite directions and the right one is the
one that matters to the user, you need to know which column is yours
before you start, otherwise you'll roll back the right change for the
wrong reason.

## the parts you don't see in the table

phases 1 to 5 are the structural changes, but a lot of work that
moved real time isn't tied to a phase number, it's per-layer
sharpening that you have to do if you want the bench to stay clean
when content gets weird, none of these are exotic, they're just
correct compiler hygiene.

**lexer.** byte-offset spans, not char offsets, every token carries a
`Span { start, end }` of byte indices, slicing source from a `&str` is
o(1) at byte boundaries, the lexer slices source dozens of times per
file (token text, code block bodies, frontmatter), if you used char
offsets it'd be o(n) per slice and you'd notice, also `memchr` for
newline scans inside multi-line constructs, hand-rolled scan loops
are slower than memchr, every time, the early lexer also dropped
whitespace as trivia, that turned out to be wrong because inline link
spacing depends on whitespace tokens (`[x](url) y` vs `[x](url)y`),
so the emit() preserves whitespace and only drops `Newline` /
`Quote`, that fix alone unblocked correct inline rendering and didn't
cost any time on the bench.

**parser.** one ast type all the way through, no mdast/hast split, no
translation pass, the table parser pulls out into its own function so
the inline parser doesn't have to special-case rows, the blockquote
parser is stack-based with depth tracking instead of recursion (`> > >
deepest` used to spawn empty blockquote levels, now it doesn't), the
list parser handles loose vs tight lists with retroactive paragraph
wrap, that means you only know a list is loose at the end but you
fix the children when you do (`ensure_loose_item` walks the list and
wraps inline content), nested lists have an `indent` parameter so the
recursive call knows where it is in the tree, lists inside
blockquotes have `parse_list_in_blockquote(ordered, depth)` because
each item line is preceded by `>` markers we have to skip before
reading the next list marker, triple emphasis (`***bold italic***`)
wraps as `<em><strong>` instead of just `<strong>`, backslash escapes
strip the backslash via `unescape_markdown`, image and link titles
split via a `split_destination_title` that walks back from the end of
`(href "title")` looking for balanced quotes, all of these are bug
fixes that came out of the side-by-side comparison with velite, none
of them moved the bench by themselves, but the combined effect is
that the parser is correct on the same fixtures velite is correct
on, which is the only honest way to compare.

**transformers.** the pipeline is `Vec<Box<dyn Transformer>>`, every
transformer takes `&self` (not `&mut self`), so a single pipeline is
shared across rayon workers, mutable per-call state lives in the
visitor passed to `walk_root`, the walker uses a `NodeAction` enum
with `Keep`, `KeepSkipChildren`, `Replace(Vec<Node>)`, `Remove`, that
covers every rewrite shape without making each transformer reinvent
splice logic, the math transformer is the interesting one, it runs a
*source-level* preprocess **before** the lexer, rewriting `$...$` and
`$$...$$` regions to `<MathMl mathml="..."/>` jsx, that's because the
parser would otherwise interpret underscores inside math as italic
markers, the only correct way to handle that is to take the math out
of the markdown stream before the parser sees it, then the html
emitter recognizes `MathMl` as a special component and pastes the
math html verbatim, escape-reversed, the math output is cached by
`(latex, display, engine)` triple in memory and on disk, warm
rebuilds skip katex entirely, the mermaid transformer is the same
pattern, render once, cache by content hash, replay on next build.

**pretty-code is the smartest one.** for a multi-theme config (one
light + one dark) the naive approach is to run syntect twice per
code block, once per theme, that doubles tokenization, syntect
tokenization is the expensive part, the colour resolution is cheap,
so the dmc highlighter tokenizes **once** and then resolves colours
**n times**, one per theme, the output emits css custom properties
(`--dmc-light`, `--dmc-dark`, `--dmc-light-bg`, `--dmc-dark-bg`) and
your stylesheet picks which mode to show, that's how you get
multi-theme highlighting at single-theme cost, the implementation is
~80 lines of joining adjacent tokens with matching styles
(`styles_match` + `join_adjacent`) so the html stays compact, output
is wrapped in `<figure data-dmc-figure>` with `<figcaption
data-dmc-title>` only when a title is set, the attribute names are
namespaced to `data-dmc-*` and `--dmc-*` instead of inheriting
shiki's or rehype-pretty-code's, dmc owns its output schema.

## where the themes and grammars come from

i didn't write a tokenizer per language, i bundled syntect's, syntect
reads `.sublime-syntax` (textmate-style) grammars and `.tmtheme`
themes, those formats have decades of community work behind them and
shiki uses the same upstream sources, so the smart move was to pull
shiki's grammar + theme bundle and convert the shapes once at build
time.

`scripts/convert-shiki-assets.mjs` does the conversion, shiki ships
grammars as json (vscode's textmate grammar shape) and themes as json
too, syntect wants `.sublime-syntax` for grammars and plist
`.tmtheme` for themes, the script walks the shiki grammar bundle,
emits sublime-syntax yaml, walks the shiki theme bundle, emits plist,
drops everything into `dmc-highlight/assets/`, then `build.rs` reads
the assets folder at compile time and generates a `Theme` enum
variant for every theme present, so adding a theme is "drop the file
in, rebuild, new variant exists".

the result is a `SyntaxBundle` global behind `OnceLock`, first call
in the process loads grammars + themes (~30ms cold, one time), every
call after is free, the binary ships ~12mb of bundled assets which
sounds like a lot but it's smaller than node's startup memory and
you get every language and theme from shiki's catalog without
shipping shiki, this is also why dmc's output looks identical to
shiki's, same grammars, same themes, same tokenization rules, just
running in rust instead of in v8.

## phase 5: stop telling the sidecar to do nothing

phase 3 left a strange shape, the rust pipeline now did syntax
highlighting natively, but the user's config still listed
`rehype-pretty-code` and `shiki` as js plugins, so the sidecar still
ran them, they were just no-ops because the ast had already been
highlighted, but a no-op rehype plugin still walks the tree, js still
parses the input, v8 still runs.

i added a thing called the **plugin gate**, when the user's config
lists a plugin whose work is owned by a native transformer
(`remark-gfm`, `remark-math`, `remark-emoji`, `rehype-pretty-code`,
`shiki`, `rehype-katex`, `rehype-mathjax`, `rehype-slug`,
`rehype-autolink-headings`), the engine strips it from the sidecar
payload at request time, the sidecar only runs plugins it actually
needs to run, and when the user's config lists *only* native-owned
plugins, the sidecar invocation is skipped entirely.

phase 5 numbers:

at 1000 files (phase 4 -> phase 5):

- native: 44.69 ms -> 44.73 ms (no change, sanity)
- sidecar + remark-gfm: 462.33 ms -> 46.01 ms (~10x)
- sidecar + pretty-code: 466.37 ms -> 44.94 ms (~10x)
- **sidecar + kitchen-sink: 1187.90 ms -> 144.77 ms (~8x)**
- velite + remark-gfm: 5910.21 ms -> 5934.00 ms (sanity)
- velite + kitchen-sink: 1368.20 ms -> 1381.46 ms (sanity)

this is the unlock, sidecar variants drop ~90%, realistic workload
1188ms -> 145ms, velite is unchanged across phases (sanity check, same
host, same fixtures, if velite had moved phase to phase something else
was contaminating the bench, it didn't, the numbers are real).

## the honest math

at phase 5, this hardware, this fixture set, 1000 files:

- native vs velite + gfm: 5934.00 ms vs 44.73 ms -> **132x**
- native vs velite + kitchen-sink: 1381.46 ms vs 44.73 ms -> **31x**
- sidecar + kitchen-sink vs velite + kitchen-sink: 1381.46 ms vs 144.77 ms -> **9.5x**

three speedup numbers, three different stories.

**132x** is real but misleading, it compares dmc's native path against
velite running gfm only, different workloads, use it for fun not for
decisions.

**31x** is the most generous honest number for a pure markdown user,
plain content without math or highlight, dmc beats velite by ~30x,
reproducible.

**9.5x** is the number i trust most, realistic kitchen-sink config,
gfm + math + highlight + mermaid, the full plugin chain real users
ship, dmc beats velite by an order of magnitude on that, gap holds
across rebuilds because both sides have warm caches.

if you only quote one number quote 9.5x, the others are accurate but
out of context, i'm not going to ship a 132x headline knowing it's not
the user's number.

## what i'm not saying

a few things that aren't in the bench, that you should know.

- **velite invocations include node startup**, every velite call
  shells out to its cli, node takes time to come up even with a warm
  pnpm store, some of velite's slowness is node not unified, if you
  ran velite as a long-lived daemon the numbers would shrink, nobody
  runs it that way in practice so i left the comparison as users see
  it.
- **numbers are from one host**, 32-core x86_64 linux, your laptop
  will produce different numbers, the shape is reproducible, the
  exact figures aren't.
- **the fixture set is mine**, lives in `examples/nextjs/content`,
  biased toward what i write, docs + code + math + occasional
  mermaid, your content distribution is probably different, re-run
  on yours before quoting.
- **dmc has fewer plugins than the unified ecosystem**, velite +
  unified gives you thousands, dmc has the ones i needed, if you need
  a custom remark plugin nobody has written in rust yet dmc still
  runs it via the sidecar, you pay the js cost for that subset only.

## three rules i'd give you

1. **optimize the column you ship, not the column that looks good**,
   the native column on dmc was fast on day one, celebrating it would
   have been honest but useless, real users run with plugins, real
   users hit the sidecar, the win was where the cost was.
2. **the first refactor often won't move the bench**, single-walk in
   phase 2 looked correct and moved the meter by 1-2%, two days for
   noise, but without it phase 3 was structurally impossible,
   sometimes the prerequisite work is invisible in timing, trust the
   model.
3. **read both numbers when one moves**, phase 3 made native 4x
   slower, naive reading calls that a regression and reverts, the
   whole story was in the next column, sidecar+kitchen-sink dropped
   67%, work moved from js to rust, if you only watch one number
   you'll roll back the right change for the wrong reason.

## this is alpha, not a 1.0

before the roadmap, the honest disclaimer, **dmc is 0.1.x**, eight
crates on crates.io plus the npm wrapper, working, published,
shipping, but it is alpha, the schema can break across minor
versions until 1.0, the cache format is keyed on the dmc version so
version bumps auto-invalidate (no manual migration needed but
rebuilds are cold once after a bump), the cli surface is stable
enough that i use it daily for gentleduck docs, but treat the rust
api as "expect rename pr's", if you adopt it now you're early, the
trade is you get the speed wins and you accept that something will
move under you between 0.1 and 1.0.

## what's next (the parts of the roadmap that matter)

i keep the full list in [`dmc-docs/guides/roadmap.md`](../dmc-docs/guides/roadmap.md),
the highlights ranked by what would actually move the bench or
unlock something new:

- **batched sidecar ipc**, today the sidecar is a long-lived ndjson
  daemon but each file is one round-trip, batching N files per
  request would cut ipc overhead linearly with batch size, this
  matters for users with many foreign plugins
- **persistent sidecar daemon across `dmc dev` runs**, today the dev
  server respawns the node child on watch restarts, keeping it warm
  across rebuilds saves the v8 startup cost on every save, the cli
  scaffolding is there, just needs the lifetime plumbing
- **syntect `.packdump` binary**, syntect supports a precompiled
  grammar dump that loads in milliseconds instead of tens of
  milliseconds, currently i load the textmate yaml on first call,
  the cold-start saving is small per process but visible on cli
  one-shots
- **pgo release build of `dmc-napi`**, profile-guided optimization
  on the napi binary, expected single-digit percent win on the hot
  walk path, free if i wire it into the prebuild matrix
- **inline code highlighting**, `` `code{:rust}` `` syntax for
  highlighting in inline code, the rendering path exists, just need
  the parser to recognize the language hint
- **diff syntax**, `+` / `-` line markers in code fences, common ask,
  cheap to add as another sink layer in the highlighter
- **`dmc inspect <file>` debug binary**, dump tokens / ast / output
  side-by-side for one file, this is what i use during parser bugs,
  shipping it as a real cli helps adopters
- **custom themes + grammars at config level**, today themes are
  baked at compile time via `build.rs`, runtime-loaded themes (point
  at a `.tmtheme` path in the config) is a small change that
  unblocks a lot of customization without rebuilding the binary
- **skip pretty-code pass when no fenced blocks**, this is the
  per-file gate i mentioned earlier, transformer is registered but
  ast walk should bail at the document level if there are zero code
  blocks, today the walker still descends, dollars are small but
  free and correct
- **tree-sitter alternative for highlighting**, semantic queries,
  faster, more accurate edges, this is a long-term swap-out, the
  api shape is the same so it's behind a feature flag when it lands
- **wasm bundle for client-side preview**, dmc-in-the-browser for
  live mdx editors, requires excluding the napi crate and the math
  engine that depends on quick-js, doable but not on the critical
  path

what's **out of scope**, forking unified to run remark/rehype
natively in rust (the sidecar exists for that, foreign plugins stay
foreign), replacing mdx with a custom format (mdx in, json out, no
new format), implementing every commonmark edge case to spec (i aim
for gfm-compatible behaviour, exotic reference-link forms handled
best-effort).

## where this goes

dmc is published, eight crates on crates.io, the npm package is
`@gentleduck/md`, `@gentleduck/md-sidecar` for whatever the plugin
gate can't strip, the bench harness is in
[`duck-benchmarks/`](../duck-benchmarks/) with raw json for every
phase, audit the numbers i quoted, that's the point, none of this
is a magic-show, the table is the table, run it on your hardware
before you trust it.

next on the list isn't another perf phase, it's making the plugin
ecosystem easier to extend so the sidecar matters less over time,
performance plateaus, api compounds, that's the long bet.

if you take one thing from this post, stop reading p50 in benchmark
posts, the median is the cost when nothing is wrong, p99 is the cost
when something predictable is wrong, that's what your users feel
every day.
