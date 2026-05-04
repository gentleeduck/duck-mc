# CLI reference

`dmc` is the binary built by `dmc-core`. Subcommands + flags below.

## `dmc build`

One-shot build. Reads config, processes every collection, writes
JSON + index.

```bash
dmc build [--config <path>] [--clean] [--strict]
```

| flag | default | use |
|------|---------|-----|
| `--config <path>` | auto-discover (`duck-md.config.{ts,js,mjs}` or `dmc.toml` in cwd) | use a non-default config path |
| `--clean` | from config (default false) | wipe output dir before build |
| `--strict` | from config (default false) | fail-on-warning |

Exit code: zero on success, non-zero on hard error (config load,
filesystem). Diagnostics emit to stderr but do not change the exit
code unless `--strict` is set.

## `dmc dev`

Long-running watch mode. See [`../dmc-core/dev-mode.md`](../dmc-core/dev-mode.md).

```bash
dmc dev [--config <path>]
```

Same `--config` flag. No `--clean` (would wipe + restart on every
edit).

## `dmc init`

Scaffold a config + sample content.

```bash
dmc init
```

Writes:

| file | content |
|------|---------|
| `duck-md.config.ts` | minimal `defineConfig` with one collection |
| `content/docs/index.mdx` | placeholder |
| `.gitignore` | append `.gentleduck/` if missing |

Skips files that already exist; never overwrites.

## Env vars

| var | default | use |
|-----|---------|-----|
| `dmc_SIDECAR` | `dmc-sidecar/index.mjs` (relative to cwd) | sidecar entry path |
| `DMC_SIDECAR_POOL_SIZE` | `min(cores, 4)` | sidecar worker pool size |
| `DMC_WATCH_DEBOUNCE_MS` | 100 | watch-mode debounce (planned) |

## Auto-discovery

The CLI searches the cwd for these in order:

1. `duck-md.config.ts`
2. `duck-md.config.js`
3. `duck-md.config.mjs`
4. `dmc.toml`

First match wins. Override via `--config <path>`.

## TS config host

`.ts` / `.js` / `.mjs` configs route through:

1. `bun <config>` (preferred, fastest)
2. `node --import tsx <config>` (fallback)

Either must be on PATH. Pure `.toml` configs never need a TS host.

## Feature flags

The CLI surface is gated by Cargo features:

| feature | enables |
|---------|---------|
| `cli` (default) | the `dmc` binary itself (clap + toml deps) |
| `watch` (default) | `dev` subcommand |

Slim install:

```bash
cargo install dmc --no-default-features --features cli
# build only, no watch
```

## Examples

```bash
# default build
dmc build

# alt config
dmc build --config configs/dev.toml

# clean rebuild + fail on warning
dmc build --clean --strict

# watch
dmc dev

# scaffold
mkdir my-site && cd my-site
dmc init
```

## Exit codes

| code | meaning |
|------|---------|
| 0 | success |
| 1 | config load failed (file missing, invalid YAML/TS) |
| 2 | filesystem error (output_dir unwritable) |
| 3 | strict mode + warnings present |

Watch mode never exits non-zero unless interrupted (Ctrl-C).
