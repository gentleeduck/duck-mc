# Roadmap

What is shipping next, what is parked, what is out of scope.

## Shipping in the current 0.1.x series

- [x] Native syntect highlighter (`pretty-code`)
- [x] Native KaTeX via quick-js (`math`)
- [x] Native MathML via pulldown-latex (`MathEngine::Mathml`)
- [x] Native emoji shortcode replacement
- [x] Native autolink-headings (replaces rehype-slug + rehype-autolink-headings)
- [x] Native GFM (parser handles tables, task lists, autolinks, strikethrough)
- [x] Persistent file cache (`<output>/.cache/dmc/`)
- [x] Persistent math cache (`<output>/.cache/math.json`)
- [x] Plugin gate strips native-owned names from sidecar payload
- [x] Multi-theme code highlighting (`--dmc-{mode}` CSS vars)
- [x] Single-tokenize multi-color highlight algorithm
- [x] Watch mode via `notify`
- [x] TS / JS / MJS config support via bun + tsx fallback
- [x] Schema validation via Zod-style descriptors
- [x] CLI (`dmc build`, `dmc dev`, `dmc init`)
- [x] napi binary (`@duck/md` on npm)
- [x] Side-by-side example apps (Next.js dmc + Next.js velite)

## Up next (0.2.x)

- [ ] Batched sidecar IPC (one round-trip per N files instead of per file)
- [ ] Persistent sidecar daemon across builds (`dmc dev` keeps Node child alive)
- [ ] Compile syntect grammars to `.packdump` binary (one-time
      startup saving)
- [ ] PGO release build of `dmc-napi`
- [ ] More platforms in CI (macOS, Windows, Linux arm64, musl)
- [ ] Inline code highlighting (`` `code{:rust}` `` style)
- [ ] Diff syntax in code blocks (`+`/`-` line markers)

## Quality of life

- [ ] `dmc inspect <file>` debug binary (token / AST / output dump)
- [ ] Per-feature opt-out flags in TS config (drop transformer without rebuilding binary)
- [ ] Custom theme + grammar at config level (no rebuild needed)
- [ ] Pluggable cache backend (memory only, redis, etc)
- [ ] tracing-style structured logs across all transformers

## Schema + validation

- [ ] More descriptor kinds (date range, url, email validators built-in)
- [ ] Per-collection schema strictness override

## Performance

- [ ] Skip `MdxBodyEmitter` when `emit_body: false` (already config-gated; verify)
- [ ] Skip pretty-code pass when no fenced code blocks in AST
- [ ] mmap source files >100 KB
- [ ] Move per-token color computation off the hot path

## Long-term

- [ ] Pluggable parser front-end (CommonMark vs GFM vs custom dialect)
- [ ] Tree-sitter alternative for syntax highlighting (semantic queries, faster)
- [ ] Browser build (Wasm bundle for client-side preview)
- [ ] Deno + Bun runtime support (npm: prefix works for consumers; not tested)

## Out of scope

- Forking unified to run remark/rehype natively in Rust. Foreign
  plugins live in the sidecar; native passes only for the high-value
  defaults.
- Replacing MDX with a custom format. dmc is MDX in, JSON out.
- Implementing every CommonMark edge case to spec. Aim for
  GFM-compatible behaviour; spec edge cases (e.g. exotic
  reference-style link forms) handled best-effort.

## Out of band wins

- Syntect maintainers ship more grammars / themes -> dmc bundle gets
  bigger automatically (just refresh the asset folder).
- KaTeX upstream gets faster -> KaTeX feature gets faster
  automatically.
- pulldown-latex matures -> MathML output quality improves.

## How to influence the roadmap

Open an issue with:

- The use case (real, not hypothetical).
- A bench number if perf-related (current vs target).
- An MDX fixture if a parser / output bug.

Speed of acceptance: bug fixes > performance > new features. New
transformers usually land as third-party crates; only the core few
ship in `dmc-transform/builtin/`.
