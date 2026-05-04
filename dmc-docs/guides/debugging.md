# Debugging

How to inspect every layer's output.

## Token stream (lexer)

The lexer ships a CLI sample binary:

```bash
echo "**hi**" | cargo run -p dmc-lexer --bin lexer -- --tokens
```

Output:

```
KIND        POS  LEN  RAW
----------  ---  ---  -----
Bold(2)    1:1    2  "**"
Text       1:3    2  "hi"
Bold(2)    1:5    2  "**"
SoftBreak  2:1    1  "\n"
Eof        2:1    0  ""
```

JSON dump:

```bash
echo "**hi**" | cargo run -p dmc-lexer --bin lexer -- --json
```

Useful when a parser output looks wrong: check the tokens first.

## AST (parser)

```bash
echo "# hi" | cargo run -p dmc-parser --bin parse -- --json
```

Outputs the `Document` as serde JSON. Pipe through `jq` for filtering:

```bash
echo "# hi" | cargo run -p dmc-parser --bin parse -- --json | jq '.children[0]'
```

## Native compile output

```bash
echo "# hi" | cargo run -p dmc-codegen --features bin --bin codegen -- --html
```

Pipes the source through lex + parse + default pipeline + HtmlEmitter.
Useful for quick "what does this MDX render to?" checks without the
napi binary.

## Programmatic compile (JS)

```ts
import { compile } from "@duck/md";

const out = compile(`---
title: Hi
---

# heading
`);

console.dir(out, { depth: null });
```

Returns the full `CompileOutput`. Inspect `frontmatter`, `content`,
`html`, `body`, `excerpt`, `metadata`, `toc`, `imports`, `exports`.

## Diagnostics

```rust
let mut diag = DiagnosticEngine::<Code>::new();
Engine::run(&cfg, None, &mut diag)?;

for d in diag.iter() {
    println!("[{}] {}: {}", d.code.code(), d.code.severity(), d.message);
    for label in &d.labels {
        println!("  at {}:{}", label.span.line, label.span.column);
    }
}
```

`Code::code()` is the canonical id ("E001", "T009"). `Code::severity()`
is `Error` or `Warning`.

## Cache hit/miss

Add a `println!` to `Collection::process` (around line 70 in
`dmc-core/src/engine/collection.rs`) for a quick check:

```rust
if let (Some(c), Some(k)) = (cache.as_ref(), cache_key.as_ref())
    && let Some(hit) = c.get(k)
{
    eprintln!("[cache hit] {}", path.display());
    return (Some(hit), local_diag_engine);
}
eprintln!("[cache miss] {}", path.display());
```

Or wipe the cache and watch the rebuild time:

```bash
rm -rf .gentleduck/.cache
time pnpm dmc build
time pnpm dmc build   # warm
```

## Sidecar protocol

```bash
node dmc-sidecar/index.mjs
{"id":1,"markdown":"# hi","remarkPlugins":[],"rehypePlugins":[]}
```

Sidecar prints `{"id":1,"html":"<h1>hi</h1>"}`. Type more requests on
new lines. dmc-core's `Stdio::null()` on stderr swallows plugin
errors; spawn manually to see them:

```bash
node dmc-sidecar/index.mjs 2>/tmp/sidecar.err <<EOF
{"id":1,"markdown":"$x$","remarkPlugins":["remark-math"],"rehypePlugins":[]}
EOF
cat /tmp/sidecar.err
```

## Tracing logs

`dmc-transform` and `dmc-core` use `tracing`-style `debug!` calls in
hot paths (where they exist). Enable:

```bash
RUST_LOG=dmc_transform=debug,dmc_core=info pnpm dmc build
```

Most modules do not yet emit logs; add `tracing::debug!` calls in
your transformer for visibility.

## Tokens not what you expect

The lexer-side `is_trivia` rule preserves `Whitespace` tokens but
drops `Newline` and `Quote`. Indented-code-block detection happens
in the parser via lookahead. If a token is missing, check the
`emit()` rule in `dmc-lexer/src/lib.rs`.

## AST not what you expect

Three suspects in order:

1. Lexer dropped a token. Dump tokens with `--tokens`.
2. Parser took a different branch. Read `parse_block` dispatch
   (`dmc-parser/src/block.rs`).
3. Transformer mutated post-parse. Run with `Pipeline::new()` (no
   defaults) to isolate.

## HTML not what you expect

1. Confirm AST is correct (parser dump).
2. Check `HtmlEmitter::open_tag` / `close_tag` for the node variant
   (`dmc-codegen/src/html.rs`).
3. For raw HTML hatches (`MathMl`, `MermaidSvg`), confirm the JSX
   attr value is the right string.

## Cache contents

Pretty-print a cache entry:

```bash
cat .gentleduck/.cache/dmc/86ee354a0cc1dfce.json | jq
```

Records carry the full velite-shaped output. Diff two builds by
diffing the JSON files.

## Bench output

```bash
cargo run --release -p dmc-core --features pretty-code --example bench
```

Writes `dmc-core/tmp/bench.json` + SVG plots. Inspect for regressions:

```bash
jq '.results[] | select(.label == "sidecar+kitchen-sink") | .points[-1]' dmc-core/tmp/bench.json
```

## Reproducing user issues

1. Get the user's MDX + config.
2. Run the lexer / parser / codegen sample binaries to isolate the
   layer.
3. Check `<output_dir>/.cache/` for stale entries from older configs.
4. Bump the cache version (`CARGO_PKG_VERSION` of `dmc-core`)
   manually if the cache is suspect.
