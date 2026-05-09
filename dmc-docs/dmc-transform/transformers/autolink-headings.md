# `autolink-headings`

Prepends an anchor `<a href="#<id>">` to every heading so the heading
itself becomes a stable link target. Mirrors `rehype-autolink-headings`
with `behavior: "prepend"`.

- **Source:** `dmc-transform/src/builtin/autolink_headings.rs`
- **Feature flag:** none
- **Config struct:** `AutolinkHeadings` (in-source; not exposed via TS yet)

## Output JSX

For `## Hello world` (id `hello-world`):

```html
<h2 id="hello-world">
  <a href="#hello-world" aria-label="Link to section" class="subheading-anchor">
    <span class="icon icon-link"></span>
  </a>
  Hello world
</h2>
```

## Knobs

| Knob | Default | Effect |
|---|---|---|
| `aria_label` | `"Link to section"` | `aria-label` on the anchor. |
| `class_name` | `"subheading-anchor"` | Class on the anchor. |
| `icon_class_name` | `"icon icon-link"` | Class on the inner `<span>` icon. |

Configurable through the Rust API today; TS surface pending. The
transformer is gated globally by `pipelineConfig.autolink_headings:
false` (or via the sidecar opt-out below).

## Order

Runs after [`assign-heading-ids`](./assign-heading-ids.md) — relies on
`Heading.id` being populated.

## Sidecar opt-out

Add `"rehype-slug"` or `"rehype-autolink-headings"` to
`markdown.preferSidecar`.
