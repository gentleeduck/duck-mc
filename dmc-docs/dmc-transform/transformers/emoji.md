# Emoji

Replaces `:shortcode:` patterns in `Node::Text` with the matching
unicode emoji. Replaces the JS plugin `remark-emoji`.

## Feature flag

`emoji` (default on). Pulls the `emojis` crate (~5 MB lookup table).

## Input

Walks `Node::Text`. Other variants (InlineCode, CodeBlock, JsxElement,
attribute values) are skipped, matching `remark-emoji` scope.

## Output

Mutates the text in place. Original string is replaced when any
shortcode matches; untouched when no match.

## Shortcode rules

- ASCII identifier between two `:`s
- Charset: `[a-z0-9_+-]+`
- Cap: 64 chars max between colons (avoids matching `1:2` ratios)
- Must resolve via `emojis::get_by_shortcode(s)`; unknown shortcodes
  pass through verbatim

## Example

Input:

```md
Ship it :rocket: :sparkles:! Ratio 1:2 stays text. ETA 13:45.
Time port :3000.
```

After emoji pass + render:

```html
<p>Ship it 🚀 ✨! Ratio 1:2 stays text. ETA 13:45.
Time port :3000.</p>
```

`1:2`, `13:45`, `:3000` do not match because the inner part is not a
valid shortcode (digits-only is allowed but unknown to the lookup
table).

## API

```rust
pub struct Emoji;

impl Transformer for Emoji {
    fn name(&self) -> &str { "emoji" }
    fn transform(&self, doc, _meta, _diag);
}
```

Path: `dmc_transform::Emoji`.

## Plugin gate

When `emoji` feature is on, `remark-emoji` is stripped from the
sidecar payload.

## Behaviour vs remark-emoji

| | dmc Emoji | remark-emoji |
|-|-----------|--------------|
| engine | static unicode table (Rust) | unified plugin (JS) |
| speed | ~5 us per text node | tens of us per node |
| coverage | full GitHub shortcode set | same |
| visual | unicode | unicode |

Identical visual output; faster path.
