use std::path::PathBuf;

use dmc_diagnostic::{Code, DiagResult};
use duck_diagnostic::{Diagnostic, diag};
#[derive(clap::Args)]
pub struct InitCmd {
  #[arg(long, default_value = "dmc.toml")]
  pub path: PathBuf,
}

impl InitCmd {
  pub fn run(self) -> DiagResult<Diagnostic<Code>> {
    if self.path.exists() {
      return Err(diag!(Code::ConfigExists, format!("refusing to overwrite existing {}", self.path.display())));
    }

    std::fs::write(&self.path, DEFAULT_CONFIG).map_err(|e| {
      diag!(
        Code::Custom { code: String::from("N001"), severity: duck_diagnostic::Severity::Note },
        format!("write error: {}", e.to_string())
      )
    })?;

    Ok(diag!(
      Code::Custom { code: String::from("N001"), severity: duck_diagnostic::Severity::Note },
      format!("wrote {}", self.path.display())
    ))
  }
}

const DEFAULT_CONFIG: &str = r#"# dmc.toml - Rust MDX compiler config.
# Every option below is commented at its default; uncomment + change to override.

# --- Engine ---------------------------------------------------------------
# Project root used to resolve relative paths in collection patterns.
# root = "."

# Where compiled JSON indexes are written. One file per collection: <output_dir>/<name>.json
output_dir = ".dmc"

# Optional: name + format of the aggregated index file (when multiple collections).
# output_name   = "index"
# output_format = "esm"   # "esm" | "json"

# Wipe output_dir before each build (also enabled by `--clean` flag).
# clean = false

# Abort on the first frontmatter validation failure (also enabled by `--strict` flag).
# strict = false

# Include rendered HTML in the per-record output (alongside `body`/`content`).
# Sidecar always emits HTML when JS plugins are configured; this is for the
# native render path.
# include_html = false

# --- Compile (markdown / mdx) ---------------------------------------------
# GitHub-Flavored Markdown extensions: tables, strikethrough, autolinks, task lists.
# markdown_gfm = true

# Native HTML emit on (auto-disabled for files that go through the sidecar).
# emit_html = true

# Native MDX body emit on.
# emit_body = true

# Run a JS minifier on the emitted MDX body (requires sidecar).
# mdx_minify = false

# MDX output format: "module" wraps body as an ES module with the imports
# rolled in; "function-body" returns just the function body string.
# mdx_output_format = "function-body"

# --- JS plugin pipelines (run via the Node sidecar) -----------------------
# Each entry is either "package-name" or ["package-name", { ...options }].
# Plugins resolve from the project's node_modules first, then sidecar's.
#
# Example:
#   markdown_remark_plugins = [
#     "remark-gfm",
#     ["remark-toc", { tight = true }],
#   ]
#   markdown_rehype_plugins = [
#     ["rehype-pretty-code", { theme = "github-dark" }],
#   ]
#
# markdown_remark_plugins = []
# markdown_rehype_plugins = []
# mdx_remark_plugins      = []
# mdx_rehype_plugins      = []

# --- Asset handling -------------------------------------------------------
# Copy files referenced by relative `![](...)` / `<a href="...">` into the
# output bundle. Requires both `output_assets` and `output_base` set.
# copy_linked_files = false
# output_assets     = "static"
# output_base       = "/"

# --- Sidecar tuning (env-only, listed for reference) ----------------------
# DMC_SIDECAR_POOL_SIZE  -> number of long-lived `node` processes (default min(cores, 4))
# dmc_SIDECAR            -> override path to dmc-sidecar/index.mjs

# --- Collections ----------------------------------------------------------
# Each collection globs files under `base_dir` matching `pattern`, compiles
# them, validates frontmatter against `schema` (optional), and writes
# `<output_dir>/<name>.json`. The schema doubles as a TypeScript source --
# `index.d.ts` ships a typed interface derived from it.
[[collections]]
name     = "docs"
pattern  = "docs/**/*.{md,mdx}"
base_dir = "."
# single = false        # if true, expect exactly one match; emit a single object instead of an array

# Frontmatter schema. dmc-schema descriptor (each node has `kind`):
# string | number | boolean | array (with `item`) | object (with `fields`,
# optional `passthrough`) | enum (with `variants`) | literal | union |
# nullable | optional | default | record | tuple | intersection |
# discriminatedUnion | isodate | path | slug | unique | file | image |
# markdown | mdx | raw | toc | metadata | excerpt | coerce.{string,number,boolean,date}
# Wrap a field in `{ kind = "optional", inner = { ... } }` to make it optional.
schema = { kind = "object", fields = { title = { kind = "string" }, description = { kind = "optional", inner = { kind = "string" } }, date = { kind = "optional", inner = { kind = "isodate" } }, draft = { kind = "optional", inner = { kind = "boolean" } }, tags = { kind = "optional", inner = { kind = "array", item = { kind = "string" } } }, author = { kind = "optional", inner = { kind = "object", fields = { name = { kind = "string" }, url = { kind = "optional", inner = { kind = "string" } } } } } } }
"#;
