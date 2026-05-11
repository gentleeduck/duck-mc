# dmc-parser roadmap

## Current status

- CommonMark spec suite: `652/652`.
- GFM spec suite: `670/670`.
- Block parsing is split across `block/{mod,list,blockquote,code,heading,html}.rs`.
- Bench baselines live in [`../BENCHMARKS.md`](../BENCHMARKS.md).

## Completed milestones

### F2. CommonMark grind `[x]`

- The CommonMark baseline moved from `118/652` to `652/652`.
- The parser/codegen grind closed container, list-looseness, raw-HTML,
  link-destination, entity-decoding, setext, and blank-line drift.

### F4. GFM runner `[x]`

- The GFM runner now passes `670/670`.
- The GFM push covered tables, task list items, bare/autolinks,
  legacy emphasis compatibility for the older fixture set, and the
  disallowed-raw-HTML render mode.

## Dialect flags

### `ParseOptions`

- `cm_strict_html_blocks`: spec-runner mode. Treat uppercase HTML-ish
  tags as CommonMark type-7 raw HTML blocks instead of MDX JSX
  components.
- `gfm_autolinks`: parse bare `http(s)://...` and `www....` runs as
  links during inline parsing. Default stays off so transform passes
  can own that behavior for MDX consumers.
- `legacy_gfm_emphasis`: normalize redundant nested emphasis so the
  legacy GFM fixture set matches without changing the CommonMark
  default parse behavior.

### `RenderOptions`

- `gfm_disallowed_raw_html`: escape the leading `<` of the GFM
  tagfilter disallowed raw-HTML set during HTML rendering.

## Remaining work

- Fuzz targets and longer fuzz runs.
- Miri / unsafe audits.
- Broader Unicode case-fold and punctuation tables.
- Remaining MDX edge completion beyond the spec suites.
- CI gates for benchmark regressions.
