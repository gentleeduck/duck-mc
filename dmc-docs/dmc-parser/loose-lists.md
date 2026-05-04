# Loose vs tight lists

Whether `<li>` children get wrapped in `<p>` depends on whether the
list is loose or tight. dmc follows CommonMark.

## Definition

A list is **loose** if any of:

- Any item contains two block-level children separated by a blank
  line.
- Any two items are separated by a blank line.

Else it is **tight**.

## Tight render

```md
- one
- two
- three
```

```html
<ul>
  <li>one</li>
  <li>two</li>
  <li>three</li>
</ul>
```

Inline content sits directly inside `<li>`. No `<p>`.

## Loose render

```md
- one

- two

- three
```

```html
<ul>
  <li><p>one</p></li>
  <li><p>two</p></li>
  <li><p>three</p></li>
</ul>
```

Each item's inline content wrapped in `<p>`. The blank lines
between items trigger looseness.

## Mixed-cause looseness

If even one item is multi-block, the entire list is loose:

```md
- one

  with paragraph

- two
- three
```

All three items get `<p>`-wrapped, including the simple ones.

## Implementation

`dmc-parser/src/block.rs::parse_list` tracks `is_loose: bool`:

```rust
let mut items: Vec<ListItem> = Vec::new();
let mut is_loose = false;
let mut prev_had_blank = false;

while let Some(line) = self.peek_line() {
    if line_is_blank() {
        prev_had_blank = true;
        self.consume_line();
        continue;
    }
    if let Some((_, _)) = parse_marker(line) {
        if prev_had_blank && !items.is_empty() { is_loose = true; }
        prev_had_blank = false;
        items.push(self.parse_list_item());
    } else if let Some(item) = items.last_mut() {
        if prev_had_blank { is_loose = true; }
        prev_had_blank = false;
        item.children.push(self.parse_block());
    } else {
        break;
    }
}

if is_loose { ensure_paragraph_wrap(&mut items); }
```

## ensure_paragraph_wrap

Retroactive: if the list ends loose but earlier items had bare
inline children, wrap them now.

```rust
fn ensure_paragraph_wrap(items: &mut [ListItem]) {
    for item in items {
        let mut new_children = Vec::new();
        let mut buf = Vec::new();
        for child in std::mem::take(&mut item.children) {
            if is_inline(&child) {
                buf.push(child);
            } else {
                if !buf.is_empty() {
                    new_children.push(Node::Paragraph(Paragraph { children: std::mem::take(&mut buf) }));
                }
                new_children.push(child);
            }
        }
        if !buf.is_empty() {
            new_children.push(Node::Paragraph(Paragraph { children: buf }));
        }
        item.children = new_children;
    }
}
```

`is_inline` matches `Text | Bold | Italic | Link | Image |
InlineCode | ...`. Block-level children (Paragraph, List, CodeBlock,
Heading, Blockquote) stay as-is.

## Why retroactive

The looseness signal can come from the last item:

```md
- one
- two

  paragraph
```

When parsing item 1 we don't know yet that item 2 is multi-block.
The retroactive wrap handles this.

## Why this matters

CSS: tight lists render compact (single line per item); loose lists
get `<p>`'s default margin. Authors expect both. Mismatching the
render breaks visual hierarchy.

## Edge cases

- Single item, multi-block: loose.
- Multi item, all single-block, no blanks: tight.
- Trailing blank lines after last item: do not trigger loose
  (CommonMark spec).
- Nested lists: loose / tight evaluated independently per nesting
  level.

## Tests

`dmc-parser/tests/loose_lists.rs`:

- tight (no blanks)
- loose (blank between items)
- loose (multi-block item)
- mixed (one multi-block triggers all-wrap)
- nested loose inside tight (nested wrapped, outer not)
- trailing blank ignored
