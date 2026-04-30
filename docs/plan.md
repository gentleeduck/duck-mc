● What miss (concrete gaps)

Lexer (5 left in Phase 1)
- L11: blockquote nesting >> — token emit not done
- L12: thematic break --- — collide w/ frontmatter delim, needs split logic
- L14: JSX fragment <> tokens — AST node exist, lexer not emit
- L21: indented 4-space code blocks — deferred (fenced cover 99%)
- L22: HTML block passthrough — lowercase tags not split from text
- L23: column tracking byte-based, not grapheme — bad error col on UTF-8

Parser
- P24: per-construct test suite missing
- P25: no recovery for unterminated JSX (synth close + keep going)
- list nesting flat — no nested list parse

Schema (Phase 7) — whole crate stub
- primitives builder missing: string/number/object/array/optional/default/max/min/regex/enum_
- rich error w/ path (frontmatter.title: exceeds 99) not done

Frontmatter (Phase 10)
- YAML parse + schema validation not wired

Transformers gap (Phase 9)
- slug transformer (parser does it, not exposed as transform)
- pretty-code meta: __rawString__/__title__/__marks__ on <code>
- component-source/preview (path/name attr → tsx rewrite + code block)

CommonMark/GFM not in scope yet (Phase 16)
- footnotes [^1], definition lists, abbreviations
- math/KaTeX $...$ $$...$$
- callouts :::tip container syntax
- emoji shortcodes :rocket:
- reference links [ref] ([ref]: url)
- angle-autolink <url> (bare-URL transform exist, angle form deferred)

Plan add features

Order by ROI:

1. Finish Phase 1 lexer — L11/L12/L14/L22/L23. Unblocks blockquote-nest + HTML pass + correct error columns.
2. Phase 7 schema crate — biggest user-visible gap. Velite parity need full primitive builder.
3. Phase 10 frontmatter YAML + validate — depend on schema. Ship together.
4. Phase 9 transformer pack — slug/pretty-code/component-source. Doc-site parity.
5. Phase 15 hardening — criterion benches (H3/H4), fuzz targets (H5/H6), feature flags gfm/mermaid/pretty-code/cli, MSRV pin.
6. Phase 16 expansion — footnotes → callouts → math → emoji → ref links. Each as new lexer module + AST variant + codegen branch.

Architecture debt flagged plan.md

- U1 sidecar spawn 115ms/file — switch to NDJSON long-lived process. ~40ms cold. Biggest perf win at scale.
- U4/U5 codegen alloc — pass &mut String writer not return String per node. 30–50% single-file speedup.
- Schema regex compile per-file — compile at config-load. chars().count() O(n) — cache len.

Bench baseline (keep eye on)

- 1 file 96L: 0.119ms (21.7× @mdx-js/mdx)
- 999 files: 46ms (159× velite)

Want me start which one? Suggest L11+L12+L14 first (small, unblock blockquote+HTML), or jump Phase 7 schema (bigger user impact).

