# Ordered list markers

How dmc parses `1.`, `2)`, and respects start values.

## Marker shapes

GFM accepts:

- `1.` `2.` `3.` ... (period)
- `1)` `2)` `3)` ... (paren)

dmc accepts both. Markers within one list must use the same
delimiter; switching delimiter starts a new list.

## Source

`dmc-parser/src/block.rs::parse_ordered_list_marker` returns
`Option<(start, delim)>`:

```rust
fn parse_ordered_list_marker(line: &str) -> Option<(u32, char)> {
    let mut chars = line.char_indices();
    let mut digits = String::new();
    while let Some((_, c)) = chars.clone().next() {
        if c.is_ascii_digit() {
            digits.push(c);
            chars.next();
        } else { break; }
    }
    if digits.is_empty() { return None; }
    let (_, delim) = chars.next()?;
    if delim != '.' && delim != ')' { return None; }
    let start = digits.parse().ok()?;
    Some((start, delim))
}
```

## Start value

The first item's number becomes the list's `start`:

```md
3. third
4. fourth
```

Renders as:

```html
<ol start="3">
  <li>third</li>
  <li>fourth</li>
</ol>
```

`start` is omitted when the first item is `1.` (HTML default). dmc
preserves whatever the user wrote.

## Subsequent items

GFM does not require sequential numbering. Both render the same:

```md
1. one
1. two
1. three
```

```md
1. one
2. two
3. three
```

dmc honours the first marker for `start` and ignores the others.
Authors who renumber visually still get the right HTML order.

## Mixing delimiters

```md
1. item
2) other
```

Renders as two separate ordered lists. The delimiter switch is the
break:

```html
<ol><li>item</li></ol>
<ol start="2"><li>other</li></ol>
```

## Loose / tight detection

Same algorithm as bullet lists (see [`loose-lists.md`](loose-lists.md)).
Blank line between items -> children wrapped in `<p>`.

## Continuation indent

Lazy continuation rules from CommonMark:

```md
1.  item with
    continuation
2.  next
```

Continuation lines must be indented to match the marker width plus
one space (4 spaces total for `1.  `). Less indent ends the item.

```rust
let marker_width = digits.len() + 1 + 1;   // digits + delim + space
let continuation_indent = marker_width;
```

## Nested lists

```md
1. outer
   - nested bullet
   - second bullet
2. next outer
```

Inner bullet list parses recursively with `indent = 3` (the marker
width of `1. `). Mixed bullet/ordered nesting works in either
direction.

## Edge cases

- `0.` is a valid start (GFM allows zero).
- Leading zeros: `01.` parses as start = 1 (digits parse decimal).
- Markers > 9 digits: capped at u32. Beyond that returns None.
- `1.foo` (no space after delim): not a list marker; treated as text.

## Tests

`dmc-parser/tests/ordered_lists.rs` covers:

- start = 1 (no `start` attr)
- start > 1 (`start="N"`)
- delim period vs paren
- delim switch starting a new list
- non-sequential numbering
- nested lists inside ordered
- loose / tight inside ordered
