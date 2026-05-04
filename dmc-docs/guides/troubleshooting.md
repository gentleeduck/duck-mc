# Troubleshooting

## Build fails with "Cannot find module '@duck/md-linux-x64-gnu'"

The platform-specific binary did not get installed. Causes:

- Postinstall blocked (`pnpm approve-builds` if pnpm is the manager).
- Lockfile stale after switching platforms.

Fix:

```bash
rm -rf node_modules pnpm-lock.yaml
pnpm install
```

## Math renders as raw `$...$` text

Source-level preprocessor did not run. Causes:

- `math` feature disabled at compile time.
- Pre-built `.node` binary from before the math feature landed.

Fix: rebuild with the `math` feature on (default). Verify with:

```bash
pnpm dmc compile --check-features
```

## Code blocks have no syntax color

`pretty-code` feature off, or theme name typo.

Fix:

```ts
prettyCode: { theme: "Catppuccin Mocha" }   // see dmc-docs/dmc-highlight/themes.md
```

Bundled list via `Theme::from_name(s).is_some()` check.

## Cache returns stale output

Fingerprint missed a config field. Or the user changed plugin-runtime
code (function refs do not show up in serde fingerprint).

Fix:

```bash
rm -rf .gentleduck/.cache
pnpm dmc build
```

## Sidecar errors

```
sidecar: <plugin>: <message>
```

means a foreign plugin failed inside the Node child. dmc-core falls
through to native HTML; the diagnostic is logged. Inspect by spawning
manually:

```bash
node dmc-sidecar/index.mjs
# paste a request, see the response
```

Or inspect stderr (dmc-core swallows it; capture by setting
`dmc_SIDECAR=node-with-stderr-pipe.mjs`).

## "globwalk error: pattern" panic

The collection `pattern` is not relative to `base_dir`. Fix:

```ts
collections: {
  docs: {
    pattern: "docs/**/*.mdx",  // relative to root, not absolute
  },
},
```

## TS config not loading

```
ts config requires `bun` or `node` w/ tsx on PATH
```

Install one:

```bash
pnpm add -g tsx
```

or use bun (`bun --version`).

## Watch mode not picking up edits

`notify` may not see edits in network-mounted dirs. Switch to polling
via env var (when implemented) or use the CLI subprocess pattern:

```bash
while true; do pnpm dmc build; sleep 1; done
```

## Out of memory with huge MDX

The lexer is byte-bound. A 10 MB MDX file allocates ~30 MB in tokens
+ AST. Split into smaller files; collection patterns can reference
multiple sub-dirs.

## Different output dmc vs velite

Visual differences are expected:

- KaTeX vs MathML rendering (`mathEngine`).
- syntect vs shiki span coalescing (cosmetic; same colors).
- dmc emits dmc-namespaced data attrs (`data-dmc-figure`, etc).

See `examples/COMPARISON.md` for the full diff matrix.

## Debug a transformer

```bash
RUST_LOG=dmc_transform=debug pnpm dmc build
```

Each built-in transformer logs entry / exit + node counts via
`tracing`. Not all have logs yet; add `tracing::debug!` calls in your
transformer for visibility.

## Reach for the lower layers

```ts
import { compile } from "@duck/md";

const out = compile(`# hi\n\n*world*`);
console.dir(out, { depth: null });
```

`compile` returns the full `CompileOutput` (frontmatter, content,
html, body, excerpt, metadata, toc, imports, exports). Useful for
debugging which stage produced an unexpected value.
