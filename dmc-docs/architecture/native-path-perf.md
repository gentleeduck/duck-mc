# Native path performance

Optimisations that target the **no-plugin** compile path (the 95% case).
This is what runs when `cfg.compile.markdown_remark_plugins` and friends
are empty -- pure Rust, no `node` child, no IPC. Already 515x faster than
velite at N=1000 on the kitchen-sink-free path; this doc is incremental
polish.

Reference numbers (this host, before changes):

| fixture | per-file native | scale N=1000 |
|---------|-----------------|--------------|
| short  (~80 B)  | 2.5 us | -- |
| medium (~1 KB)  | 33 us  | -- |
| heavy  (~2 KB)  | 60 us  | -- |
| long   (~80 KB) | 2.96 ms | -- |
| native @N=1000  | -- | 11.9 ms (84k files/sec) |

Re-bench with `cargo run --release --example bench` after each unit and
update this table.

## Inventory of remaining wins

Ranked by impact-vs-effort. Each unit is independently shippable.

### N1. Hoist `Pipeline::with_defaults()` out of per-file loop

- **File**: `dmc-core/src/engine/collection.rs`
- **Today**: builds 3 boxed transformers per file inside `paths.par_iter`.
- **Change**: build pipeline once before the loop; clone or share via `Arc`. `CopyLinkedFiles` currently captures `path.parent()` -- refactor that transformer to take parent dir per call.
- **Expected**: ~300 ns x N files saved. Material at scale (~300 us at N=1000).
- **Effort**: 30 min.

### N2. Pre-allocate `Accumulator` buffers

- **File**: `dmc-core/src/engine/accumlator.rs`
- **Today**: `String::new()` for `plain`, `Vec::new()` for `toc_flat`.
- **Change**: `String::with_capacity(source.len())` for `plain`, `Vec::with_capacity(source.len() / 100)` for `toc_flat`.
- **Expected**: 5-10% on long fixtures; reduces realloc + memcpy cycles as buffers grow.
- **Effort**: 5 min.

### N3. Cache `Heading::slug()` on the heading capture tuple

- **Files**: `dmc-core/src/engine/accumlator.rs`, `dmc-codegen/src/html.rs`, `dmc-codegen/src/mdx.rs`
- **Today**: each emitter calls `h.slug()` independently -- two slug computations per heading.
- **Change**: `Accumulator::in_heading` already stores `(level, slug)`. Pass slug down to other sinks via shared state on `WalkCtx`, OR keep as-is and accept the duplication.
- **Cleaner alternative**: memoise on `Heading` struct via `OnceCell<String>`. Adds a field; saves the second slug computation.
- **Expected**: small (~10% on heading-heavy docs). Skip unless slugify shows up in flamegraph.
- **Effort**: 1 hour (touches every Heading callsite if memoised).

### N4. `Cow<str>` from `escape_text` / `escape_attr`

- **File**: `dmc-codegen/src/escape.rs`
- **Today**: every Text node escape allocates a fresh `String`, even when no special chars exist.
- **Change**: return `Cow<'_, str>`. Borrowed when nothing escaped, owned otherwise. Most prose has no special chars.
- **Expected**: one allocation per Text leaf saved -- big on text-heavy fixtures. ~10-15% on long.
- **Effort**: 1 hour. Touches every callsite (push_str now needs `&*cow`).

### N5. Replace `format!("...")` with `push_str` + `itoa` in HtmlEmitter

- **File**: `dmc-codegen/src/html.rs`
- **Today**: `format!("<h{} id=\"{}\">", h.level, escape_attr(&h.slug()))` allocates an intermediate String.
- **Change**: direct `push_str("<h"); itoa::fmt(out, level); push_str(" id=\""); ...`. Skip the format! string allocation entirely.
- **Expected**: per-tag allocation gone. Big on long fixtures.
- **Effort**: 2 hours. Tedious -- every `format!` in `open_tag`, `inline_table`, `code_block`, etc.

### N6. Early-bail transformers

- **File**: `dmc-transform/src/pipeline.rs`, all builtins
- **Today**: `CodeImport`, `BareUrlAutolink`, `AutolinkHeadings` each walk the full AST every compile.
- **Change**: add `fn applies(&self, doc: &Document) -> bool` to `Transformer` trait (default `true`). Each builtin pre-scans cheaply. `CodeImport::applies` returns false if no `CodeBlock` has `file=` attr. `BareUrlAutolink::applies` returns false if no `Text` contains `://`. `AutolinkHeadings::applies` returns false if no `Heading`.
- **Expected**: tiny files where no transformer applies -> 3 walks become 0 walks. ~30-50% on `# Hello` style fixtures.
- **Effort**: 4 hours. Trait change ripples to every builtin.

### N7. Reuse `Arc<SourceMeta>` per collection

- **File**: `dmc-core/src/engine/collection.rs`, `dmc-core/src/engine/compile.rs`
- **Today**: each `Compiler::compile_with_pipeline` builds `Arc::from(SourceMeta { path: path.display().to_string(), ... })`. Allocation per file.
- **Change**: build SourceMeta with `Arc<Path>` instead of `Arc<str>` so cloning is cheap. Or pre-canonicalise paths and store `Arc<SourceMeta>` per file in the par_iter map closure.
- **Expected**: <100 ns / file. Material only at very large N.
- **Effort**: 1 hour.

### N8. Walker tuple-of-sinks (drop dyn dispatch)

- **Files**: `dmc-codegen/src/lib.rs`, `dmc-core/src/engine/compile.rs`
- **Today**: `Walker::walk` takes `&mut [&mut dyn NodeSink]` -- vtable lookup per enter/leave per sink per node. ~5 ns x 6 dispatches x N nodes.
- **Change**: introduce `trait Sinks` impl'd for tuples `(A,)`, `(A, B)`, `(A, B, C)`. Walker becomes generic over `S: Sinks`. Caller passes `(&mut acc, &mut Some(html), &mut Some(body))`. Monomorphised -> direct call.
- **Expected**: ~250-450 us saved on long fixtures. Closes most of the gap vs the pre-single-walk 4-walk path.
- **Effort**: 3 hours. Need `impl<S: NodeSink> NodeSink for Option<S>` to handle optional sinks cleanly.

### N9. Iterative walker (no recursion)

- **File**: `dmc-codegen/src/lib.rs`
- **Today**: `walk_node` is recursive. Each frame allocates a `WalkCtx`.
- **Change**: explicit `Vec<(node, ctx, child_idx)>` stack inside `walk`. Pop, descend, push children. Avoids function call frames + makes a future "skip subtree" API trivial.
- **Expected**: 20-30% on long fixtures (3000+ nodes). Risky -- harder to reason about.
- **Effort**: 1 day.

### N10. Zero-copy AST (Text + InlineCode borrow into source)

- **Files**: `dmc-parser/src/ast/node.rs`, every consumer
- **Today**: `Text { value: String }` -- parser allocates per Text node.
- **Change**: `Text<'src> { value: &'src str }` borrowed into the source string. Source must outlive the Document.
- **Expected**: huge on long fixtures. AST allocation drops to ~zero.
- **Effort**: weeks. Lifetime tax across every crate. Tests, codegen, transforms all need to thread `'src`.

## Ruled out (don't pursue)

- **SIMD lex**: parser already at 36 us. <5% upside.
- **bumpalo arena allocator**: parse cost dwarfed by codegen + serialisation.
- **mmap source files**: per-file `read_to_string` of <10 KB is microseconds.
- **Custom JSON serializer**: `serde_json` already fast; output writes are I/O bound.

## Recommended ordering

1. N1 + N2 + N7 (~1 hour total, free wins)
2. N4 (Cow escape)
3. N8 (tuple sinks, biggest single-walk recovery)
4. N5 (push_str + itoa)
5. N6 (early-bail transformers)
6. N3 (slug cache) -- only if heading-slugify shows in flamegraph
7. N9 (iterative walker) -- only if recursion overhead dominates
8. N10 (zero-copy AST) -- last resort, biggest invasiveness

After each unit, re-bench and update the table at the top.

## Acceptance gate

After N1-N8 land:
- per-file short fixture: <= 1.5 us (current 2.5 us)
- per-file long fixture: <= 2.0 ms (current 2.96 ms)
- native @N=1000: <= 8 ms (current 11.9 ms)

If the bench misses a target by >20%, revert the unit -- the cleanliness was free, the perf was the whole point.
