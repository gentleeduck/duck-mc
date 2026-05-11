# `emoji`

Replaces `:shortcode:` markers in text with their unicode glyphs.
Mirrors `remark-emoji` with the github-emoji shortcode set.

- **Source:** `dmc-transform/src/builtin/emoji.rs`
- **Feature flag:** `emoji`
- **Config:** none (on/off only)

## Examples

| Source | Output |
|---|---|
| `:smile:`     | 😄 |
| `:rocket:`    | 🚀 |
| `:sparkles:`  | ✨ |

Unknown shortcodes pass through unchanged. Shortcodes inside fenced
code blocks or inline code spans are left as literal text.

## Toggle

```ts
import { defineConfig } from '@gentleduck/md/config'

export default defineConfig({
  markdown: {
    // No `emoji` slot today - controlled via sidecar opt-out + feature flag.
  },
})
```

Disable by listing `"remark-emoji"` in `markdown.preferSidecar` or
building dmc without the `emoji` Cargo feature.

## Sidecar opt-out

`"remark-emoji"` in `markdown.preferSidecar` drops the native pass and
defers to the JS plugin.
