# dmc CLI

`cargo install` builds a `dmc` binary. Used via `pnpm dmc <subcommand>`
or directly.

## Subcommands

```
dmc <command> [flags]
```

| command | use |
|---------|-----|
| `build` | one-shot build using `dmc.toml` or `duck-md.config.{ts,js,mjs}` |
| `dev`   | watch mode; rebuild on file change via `notify` |
| `init`  | scaffold a new `duck-md.config.ts` + sample content |

## Common flags

| flag | meaning |
|------|---------|
| `--config <path>` | override config path |
| `--clean` | force clean output dir before build (overrides config) |
| `--strict` | fail-on-warning |

## Feature flags

`dmc-core` Cargo features that affect the binary:

| feature | default | use |
|---------|---------|-----|
| `cli` | on | enables the CLI binary itself (clap + toml deps) |
| `watch` | on | enables `dev` (notify + notify-debouncer-mini) |
| `pretty-code` | on | forwards to `dmc-transform/pretty-code` |
| `math` | on | forwards to `dmc-transform/math` |
| `emoji` | on | forwards to `dmc-transform/emoji` |

To build a slim variant:

```bash
cargo install dmc --no-default-features --features cli,watch
```

Drops syntect bundle, KaTeX engine, and emoji table. Useful when you
only need the markdown -> HTML core path.

## Watch mode

```bash
dmc dev
```

Sets up:
- `notify` watcher rooted at `cfg.root`
- `notify-debouncer-mini` to coalesce rapid edits
- on debounce fire: rerun `Engine::run` with the existing config

The persistent cache means only changed files re-compile.

## Init

```bash
dmc init
```

Writes:
- `duck-md.config.ts` from a template
- `content/docs/index.mdx` placeholder
- `.gitignore` entry for `.gentleduck/`

Skips files that already exist.
