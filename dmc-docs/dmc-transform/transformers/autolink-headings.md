# AutolinkHeadings

Wraps every `Heading`'s children in a `Link` to its own `#id`.
Replaces the JS plugins `rehype-slug` + `rehype-autolink-headings`.

## Feature flag

Always on (no Cargo gate).

## Input

Every `Node::Heading { level, children, span }`.

## Output

The heading still has `level` + `span`, but its `children` becomes a
single `Link` wrapping the original inline content:

```rust
Heading {
    level: 2,
    children: vec![
        Node::Link(Link {
            href: format!("#{slug}"),
            title: aria_label,        // optional
            children: original_inline,
            span,
        })
    ],
    span,
}
```

`HtmlEmitter` then renders it as:

```html
<h2 id="{slug}">
  <a href="#{slug}" title="{aria_label}" class="subheading-anchor">
    Original heading inline content
  </a>
</h2>
```

The `id` attribute on the `<h2>` comes from `Heading::slug()` via
`slug::slugify`. Slugs are deterministic on the rendered plain text
of the inline children.

## Configuration

```rust
pub struct AutolinkHeadings {
    pub aria_label: String,    // default "Link to section"
}

impl AutolinkHeadings {
    pub fn new() -> Self;
}
```

Path: `dmc_transform::AutolinkHeadings`. The `aria_label` flows
through as the link's `title` attribute.

## Plugin gate

`rehype-slug` and `rehype-autolink-headings` are stripped from the
sidecar payload (always; this transformer is always on).

## Example

Input:

```md
## Hello, *World*!
```

After AutolinkHeadings + render:

```html
<h2 id="hello-world">
  <a href="#hello-world" title="Link to section" class="subheading-anchor">
    Hello, <em>World</em>!
  </a>
</h2>
```

## Slug source

`Heading::slug()` flattens inline children to plain text and runs
`slug::slugify`:

```rust
fn plain_text(nodes: &[Node]) -> String {
    let mut s = String::new();
    for n in nodes {
        match n {
            Node::Text(t) => s.push_str(&t.value),
            Node::Bold(i) | Node::Italic(i) | Node::Strikethrough(i) => {
                s.push_str(&Self::plain_text(&i.children))
            }
            Node::Link(l) => s.push_str(&Self::plain_text(&l.children)),
            Node::InlineCode(c) => s.push_str(&c.value),
            _ => {}
        }
    }
    s.trim().to_string()
}
```

Includes inline code text. Skips JSX and images.

## Why one transformer for slug + autolink

`rehype-slug` and `rehype-autolink-headings` are sequential in the JS
ecosystem (slug must run first). Combining into one Rust pass avoids
double-traversal and keeps the slug computation co-located with the
wrap logic.
