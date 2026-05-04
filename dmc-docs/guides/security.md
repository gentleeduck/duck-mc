# Security

Threat model + safe defaults for processing untrusted MDX.

## Trust assumptions

dmc assumes:

- Source MDX is trusted (authored by repo maintainers).
- Foreign plugins listed in config are trusted.
- Output JSON is consumed by trusted code that may render `html`
  via `dangerouslySetInnerHTML` / `{@html ...}` / `set:html`.

If your input is user-generated (forums, comments, wiki edits), you
need extra layers.

## Risks per stage

| stage | risk | mitigation |
|-------|------|-----------|
| MDX source | malicious JSX expression `{eval(...)}` | sanitise on input; do not run MDX runtime against untrusted input |
| copy-linked-files | path traversal `../../etc/passwd` | restrict to files under `cfg.root`; the transformer canonicalises and rejects symlinks pointing outside |
| Mermaid renderer | `mmdc` runs a headless browser | `mmdc` already sandboxes; risk is denial-of-service via huge graphs |
| KaTeX engine | quick-js sandboxes the JS heap | low risk; KaTeX itself is dependency-free |
| sidecar plugins | arbitrary remark/rehype plugin code runs in Node child | trust the plugin code as you would any npm dep |
| HTML output | XSS via `<script>` / `<img onload="...">` | dmc preserves user-authored JSX verbatim; sanitise downstream if input is untrusted |

## Untrusted input

For user-generated MDX:

1. Run input through a sanitiser before compile (e.g. `sanitize-html`
   on the markdown source, or post-process the rendered HTML with
   `rehype-sanitize` via the sidecar).
2. Disable JSX support: dmc currently always enables JSX via the
   parser. Strip `<` from input, or run a pre-pass that escapes
   user-supplied HTML/JSX.
3. Drop `JsxExpression` nodes via a custom transformer:

```rust
struct DropJsxExprs;
impl Visitor for DropJsxExprs {
    fn visit_node(&mut self, node: &mut Node) -> NodeAction {
        match node {
            Node::JsxExpression(_) | Node::JsxElement(_) | Node::JsxSelfClosing(_) | Node::JsxFragment(_) => {
                NodeAction::Remove
            }
            _ => NodeAction::Keep,
        }
    }
}
```

Add to the pipeline; output is JSX-free.

## XSS

dmc's `escape_text` and `escape_attr` cover `<`, `>`, `&`, and `"`.
But:

- The `MathMl` and `MermaidSvg` raw-HTML hatches paste the attr value
  verbatim. Inputs to these come from the Math / Mermaid transformers
  (Rust code, trusted). User-authored `<MathMl>` / `<MermaidSvg>` JSX
  in source MDX would also paste verbatim; treat them as code (which
  they are) and only allow them in trusted MDX.
- Inline JSX (`<Comp>...</Comp>`) is preserved by `MdxBodyEmitter` for
  runtime React/MDX. The HTML emitter renders the tag verbatim in
  HTML form. For untrusted input, strip JSX (see above).

## Path traversal

`copy-linked-files`:

- Paths get resolved relative to the source file's parent dir.
- Symlinks pointing outside `cfg.root` are not followed.
- Absolute paths (`/etc/...`) are not copied; left as-is in `src` /
  `href`.

If you ship dmc to render user-supplied MDX with referenced files,
restrict the source dir and the asset dir to your sandbox; do not
allow user input to set `output.assets` etc.

## Resource exhaustion

dmc uses rayon for per-file parallelism. A large content set with
`mathEngine: "katex"` can saturate cores. Cap concurrency by
restricting the rayon thread pool (`RAYON_NUM_THREADS=4`).

Math expressions in user input should be length-limited. KaTeX has
a `maxExpand` macro-expansion cap; dmc passes default `Opts` so this
is the KaTeX default.

Mermaid rendering shells out to `mmdc` per unique source. A user
posting many distinct mermaid graphs can DoS the renderer. Cache
helps but does not bound concurrent renders. Drop the `mermaid`
feature for user-generated input pipelines.

## Cache poisoning

The persistent file cache is keyed by `blake3(version + source +
path + cfg)`. A user cannot poison another user's cache because:

- file paths are absolute on disk; user inputs do not influence them
- `version` blocks across-version collisions
- `cfg` blocks across-config collisions

If you share a cache directory between users / projects, use
separate `output_dir` per user. Do not share cache dirs across
trust boundaries.

## Secrets

dmc never reads env vars beyond:

- `dmc_SIDECAR` (sidecar entry path)
- `DMC_SIDECAR_POOL_SIZE`
- `RUST_LOG` (tracing)

It does not exfiltrate; output goes only to `output_dir`. Verify by
auditing the `engine::run` flow.

## Sidecar process

The sidecar is `node` running JS. By default `dmc-sidecar/index.mjs`
relative to cwd. If you ship dmc into a hostile environment, set
`dmc_SIDECAR` to a known-good absolute path; otherwise an attacker
who controls cwd could swap the file.

## TS config host

`.ts` configs run via `bun` or `node + tsx`. The user's config file
is JS code: it runs with full Node permissions. Only run dmc against
configs you trust. The config file is not a sandboxed format.

## Signed releases

(Future) The published `*.node` napi binary ships in the npm tarball.
Pin a specific version + verify the integrity hash in your lockfile.
