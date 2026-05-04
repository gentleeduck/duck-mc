# Mermaid

Renders `code lang=mermaid` blocks to inline SVG via the external
`mmdc` CLI (`@mermaid-js/mermaid-cli`).

## Feature flag

`mermaid` (default on). Requires `mmdc` on PATH at runtime.

## Input

Any `Node::CodeBlock { lang: Some("mermaid"), value, .. }`.

## Output

```html
<MermaidSvg svg="<svg ...>...</svg>"/>
```

JSX self-closing element with the SVG verbatim in the `svg` attribute.
The `HtmlEmitter` recognises `MermaidSvg` and pastes the attribute
value raw.

## Cache

Two-level:

- L1: in-memory `Mutex<HashMap<u64, String>>` keyed by
  `default_hasher(source)`
- L2: optional disk cache when `Mermaid::with_output(p)` is set; one
  `<key>.svg` file per render

```rust
pub struct Mermaid {
    pub output_dir: Option<PathBuf>,
    cache: Mutex<HashMap<u64, String>>,
}

impl Mermaid {
    pub fn new() -> Self;                          // Default
    pub fn with_output(p: impl Into<PathBuf>) -> Self;
    pub fn render_cached(&self, source: &str) -> Result<String, String>;
}
```

Path: `dmc_transform::Mermaid`.

## CLI invocation

```bash
mmdc --input - --output - --outputFormat svg
```

Source on stdin; SVG on stdout. Errors captured from stderr.

## Availability check

```rust
fn mmdc_available() -> bool {
    *MMDC_AVAILABLE.get_or_init(|| {
        Command::new("mmdc")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    })
}
```

Probed once per process. If missing, the transformer no-ops with a
single `TW001 MmdcUnavailable` warning per build (so users know to
install).

## Failure modes

| failure | code | severity |
|---------|------|----------|
| `mmdc` not on PATH | `TW001 MmdcUnavailable` | warning |
| `mmdc` exit non-zero | `T009 MermaidRenderFailed` | error (per block) |
| stdin/stdout pipe error | `T009 MermaidRenderFailed` | error |

Per-block failures leave the original code block intact; build
continues.

## Example

Input:

````md
```mermaid
graph TD
  A --> B
```
````

After Mermaid pass + render:

```html
<svg xmlns="http://www.w3.org/2000/svg" ...>
  <!-- mermaid-rendered graph -->
</svg>
```

## Install

```bash
npm i -g @mermaid-js/mermaid-cli
```

mmdc bundles puppeteer + a headless browser for rendering. Heavy
install (~150 MB) but the SVG output is high quality.

## Why a sidecar process

Mermaid is a JS library plus a browser-based layout engine. Porting
to Rust is impractical. Cache mitigates the cost (most builds reuse
identical diagrams).
