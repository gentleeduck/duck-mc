# `disable-gfm`

Strips GitHub-flavored-markdown extensions (tables, strikethrough,
task lists, autolink-literals) from the parsed document. Only added
to the pipeline when the user opts out of GFM.

- **Source:** `dmc-transform/src/builtin/disable_gfm.rs`
- **Feature flag:** none
- **Config:** `markdown.gfm: false` (top-level, not a transformer-local knob)

## Behavior

Walks the AST and removes / unwraps any node produced by GFM
extensions:

- `Table` nodes → flattened to plain paragraphs of cell content.
- `Strikethrough` (`~~…~~`) → unwrapped into plain text.
- `TaskList` checkboxes → stripped, leaving just the list item text.
- Autolink-literals → reverted to plain text.

The dmc parser handles GFM natively, so disabling it here means the
output matches CommonMark byte-for-byte.

## Sidecar opt-out

`"remark-gfm"` in `markdown.preferSidecar` keeps GFM but routes through
the JS sidecar instead of the native parser path.
