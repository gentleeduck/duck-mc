# `assign-heading-ids`

Populates `Heading.id` on every `<h1>…<h6>` AST node using a
github-slugger algorithm with per-document deduplication.

- **Source:** `dmc-transform/src/builtin/assign_heading_ids.rs`
- **Feature flag:** none (always built)
- **Config:** none

## Slug algorithm

1. Collect heading text content (recursively, skipping JSX attribute values).
2. Lowercase. Strip `.` and `'`. Replace runs of non-alphanumeric chars with `-`.
3. Trim leading/trailing `-`.
4. Per-document dedupe: collisions get `-1`, `-2`, … suffixes in source order.

Matches `rehype-slug` + `github-slugger` byte-for-byte for the typical
heading set.

## Why this runs first

Every downstream transformer that needs heading ids
([`autolink-headings`](./autolink-headings.md), the TOC accumulator, the
HTML/MDX emitters) reads `Heading.id` directly. Assigning once up-front
avoids each consumer maintaining its own slugger state.

## Sidecar opt-out

Set `"rehype-slug"` in `markdown.preferSidecar` (also drops
[`autolink-headings`](./autolink-headings.md)).
