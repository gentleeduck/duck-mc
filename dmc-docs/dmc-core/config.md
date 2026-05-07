# Configuration

Two structs: `EngineConfig` (top-level) embeds `CompileConfig`
(per-render).

## `EngineConfig`

```rust
pub struct EngineConfig {
    pub root: PathBuf,
    pub output_dir: PathBuf,
    pub output_name: Option<String>,
    pub output_format: Option<String>,  // "esm" | "cjs"
    pub clean: bool,
    pub strict: bool,
    pub collections: Vec<Collection>,
    pub include_html: bool,
    pub cache_enabled: bool,            // default true
    pub compile: CompileConfig,
}
```

| field | meaning |
|-------|---------|
| `root` | content root that collection patterns resolve from |
| `output_dir` | where `<name>.json` + `index.{js,d.ts}` land |
| `output_format` | `"esm"` (default) or `"cjs"` for the index |
| `clean` | wipe `output_dir` before build |
| `strict` | fail-on-warning (caller-driven; engine just emits) |
| `collections` | one entry per `<name>.json` to emit |
| `include_html` | force HTML field on every record (useful for SSR) |
| `cache_enabled` | persistent cache (per-file + math) |
| `compile` | flattened `CompileConfig` |

`#[serde(default)]` so missing fields use defaults.

## `CompileConfig`

```rust
pub struct CompileConfig {
    pub markdown_gfm: bool,
    pub emit_html: bool,
    pub emit_body: bool,
    pub mdx_minify: bool,
    pub mdx_output_format: Option<String>,  // "function-body" | "module"
    pub markdown_remark_plugins: Vec<Value>,
    pub markdown_rehype_plugins: Vec<Value>,
    pub mdx_remark_plugins: Vec<Value>,
    pub mdx_rehype_plugins: Vec<Value>,
    pub copy_linked_files: bool,
    pub output_assets: Option<String>,
    pub output_base: Option<String>,
    pub pretty_code: Option<PrettyCodeOptions>,
    pub math_engine: Option<MathEngine>,
    pub force_sidecar: bool,
    pub prefer_sidecar: Vec<String>,
}
```

| field | use |
|-------|-----|
| `markdown_gfm` | dmc parser handles GFM. Set false to disable tables/strike/task lists |
| `emit_html` | run `HtmlEmitter` |
| `emit_body` | run `MdxBodyEmitter` |
| `mdx_minify` | minify the JS body via swc-style minifier |
| `mdx_output_format` | wrap body as `function-body` or full `module` |
| `markdown_*_plugins` / `mdx_*_plugins` | foreign unified plugins for the sidecar |
| `copy_linked_files` | copy `src=`/`href=` assets to `output_assets` |
| `pretty_code` | theme + multi-mode override; `None` uses bundled defaults |
| `math_engine` | `Katex` (default) or `Mathml` |
| `force_sidecar` | global plugin-gate bypass; every JS plugin runs in sidecar, every native transformer dropped from pipeline |
| `prefer_sidecar` | per-plugin gate bypass; names listed here run in sidecar, matching native transformer dropped |

### Plugin gate override

The plugin gate (see [`compile.md`](compile.md#plugin-gate)) strips
plugin names from the sidecar payload when a native transformer
already does the work. To force the JS implementation instead:

- **One specific plugin:** add its name to `prefer_sidecar`. The
  gate keeps it in the payload AND `pipeline_config()` drops the
  matching native transformer (no double work).
- **Every plugin:** set `force_sidecar = true`. Equivalent to
  `prefer_sidecar` listing every recognised name.

Recognised names mapped to the native transformer they replace:

| name | native transformer dropped |
| --- | --- |
| `remark-gfm` | parser GFM behaviour (`markdown_gfm = false`) |
| `remark-math`, `rehype-katex`, `rehype-mathjax` | `Math` |
| `remark-emoji` | `Emoji` |
| `rehype-pretty-code`, `shiki` | `PrettyCode` |
| `rehype-slug`, `rehype-autolink-headings` | `AutolinkHeadings` |

Every native transformer is gated by an `Option<bool>` field on
`PipelineConfig`: `emoji`, `autolink_headings`, `math`,
`pretty_code_enabled`. `CompileConfig::pipeline_config()` flips
those to `Some(false)` based on `prefer_sidecar` / `force_sidecar`
so the matching transformer is not pushed into
`Pipeline::with_defaults_for(cfg)`.

## TOML config

```toml
root = "content"
output_dir = ".gentleduck"
clean = false
include_html = true
cache_enabled = true

[compile]
markdown_gfm = true
emit_html = true

[[collections]]
name = "doc"
pattern = "docs/**/*.mdx"
base_dir = "content"
single = false
```

## TS config

```ts
import { defineConfig, s } from "@gentleduck/md";

export default defineConfig({
  root: "content",
  output: { data: ".gentleduck", html: true },
  collections: {
    docs: {
      name: "doc",
      pattern: "docs/**/*.mdx",
      schema: s.object({
        title: s.string().max(99),
        description: s.string().optional(),
      }),
    },
  },
});
```

`EngineConfig::load_ts` spawns `bun` first, falls back to `node + tsx`.
The script `dmc-core/scripts/load-config.mjs` imports the user config,
serialises to JSON, and the Rust side parses it.

## Loading

```rust
pub(crate) fn load(config_path: &PathBuf) -> std::io::Result<EngineConfig>;
```

Routes by extension:

| ext | loader |
|-----|--------|
| `.toml` | `toml::from_str` |
| `.ts` / `.js` / `.mjs` | spawn TS host, parse JSON output |

Anything else returns `InvalidData`.
