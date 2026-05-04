# Examples

Apps in `examples/` show dmc + alternatives side-by-side. Pick one
that matches your stack.

## Layout

```
examples/
|- nextjs/             dmc + Next.js (App Router) demo
|- nextjs-velite/      velite + Next.js (same content) for parity check
|- acme-docs/          reference documentation site (in-progress)
`- web/                marketing / docs site
```

## nextjs

`pnpm --filter dmc-nextjs dev`

- App Router
- `dmc.config.ts` at repo root
- Content lives in `examples/nextjs/content/docs/*.mdx`
- Layout reads `.dmc/Doc.json`
- Kitchen-sink page exercises every transformer (math, mermaid, code
  highlighting, npm-command, emoji, autolinks, tables, blockquotes,
  task lists, footnotes, JSX components)

Use this as the reference for wiring dmc into a real Next.js app.

## nextjs-velite

`pnpm --filter velite-nextjs dev`

- Same content as `nextjs/` (content folder is symlinked or copied)
- Uses `velite.config.ts` + `velite` build pipeline
- Demonstrates the parity target: dmc output should look the same
  as velite's for the shared content

Run both side-by-side at different ports for visual diff during
development.

## acme-docs

Documentation site template. Tracks the dmc roadmap; not yet
production-ready.

## web

Marketing site (homepage + docs). Production deploy target. Uses
the dmc CLI rather than the napi import, so the pipeline runs
out-of-process.

## Running both nextjs apps

```bash
# terminal 1
pnpm --filter dmc-nextjs dev   # localhost:3000

# terminal 2
pnpm --filter velite-nextjs dev   # localhost:3001
```

Open the same route in both; visual diff for parity. Useful when
adding parser fixes or transformer changes.

## Bench against the example

The bench harness reads `examples/nextjs/content` as its corpus:

```bash
cargo run --release -p dmc-core --features pretty-code --example bench
```

Output: `dmc-core/tmp/bench.json` + plots. Compares dmc vs the
velite shell-out path on the same content set.

## Add a new example

1. `mkdir -p examples/<name>`
2. `pnpm init` inside; add to root `pnpm-workspace.yaml`.
3. Drop content under `examples/<name>/content/`.
4. Add `dmc.config.ts` pointing at the content.
5. Wire your framework (Next.js, Astro, Vite, SvelteKit, Remix).
6. Add a `dev` script. Verify `pnpm --filter <name> dev` works.

## Why side-by-side examples

Shipping a new parser fix or transformer often means matching some
subset of velite / Contentlayer / shiki / KaTeX behaviour. The
side-by-side apps catch parity regressions that unit tests miss.

Snapshot tests cover the AST and emitted HTML. The example apps
catch the bits that depend on browser rendering (CSS, fonts, KaTeX
stylesheet, mermaid SVG sizing).
