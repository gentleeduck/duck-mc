# DisableGfm

Strips GFM constructs (tables) when `markdown_gfm: false`. Used by
sites that want a pure CommonMark subset without GFM tables.

## Feature flag

Always on. Activated only when `cfg.markdown_gfm == Some(false)`.

## Input

`Node::Table { children, .. }`.

## Behaviour

Replace the `Table` with a `Paragraph` containing the rendered cell
text joined by spaces. Effectively disables tables; downstream
emitters never see `Node::Table`.

## API

```rust
pub struct DisableGfm;

impl Transformer for DisableGfm {
    fn name(&self) -> &str { "disable-gfm" }
}
```

Path: `dmc_transform::DisableGfm`.

## Why a transformer

The dmc parser handles GFM by default (tables, strike, task lists,
autolinks). Adding `DisableGfm` as a post-pass is simpler than a
config flag plumbed through the parser; downstream code never has to
branch.

Note: only tables are currently stripped. Strikethrough, task lists,
and bare-URL autolink remain because they are inline-level and harder
to undo cleanly. Disable those by stripping the corresponding plugins
from the source MDX.

## Example

Source:

```md
| a | b |
|---|---|
| 1 | 2 |
```

With `markdown_gfm: false`, after pass:

```md
a b
1 2
```

(Both rows joined into one paragraph; cells space-separated.)

## When to use

- Strict CommonMark output for tooling that does not understand GFM.
- Migrating content from a non-GFM source where existing tables are
  noise.
- Tests that need deterministic non-table output.

Most users keep `markdown_gfm: true` (default).
