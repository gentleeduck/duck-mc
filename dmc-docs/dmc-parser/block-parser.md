# Block parser

Source: `dmc-parser/src/block.rs`. All methods hang off
`impl<'eng, 'tokens> Parser<'eng, 'tokens>`.

## Dispatch (`parse_block`)

`parse_block` peeks one token and routes to a specific block
handler. Two pre-checks fire before the main `match`:

### 1. Indented code probe

```rust
let is_indented = matches!(
    self.peek(),
    Some(t) if matches!(t.kind, TokenKind::Whitespace) && t.raw.starts_with("    ")
);
if is_indented {
  let next_kind = self.tokens.get(self.pos + 1).map(|t| t.kind.clone());
  if !matches!(next_kind, Some(TokenKind::UnorderedListItem) | Some(TokenKind::OrderedListItem)) {
    return Some(self.parse_indented_code());
  }
}
```

A `Whitespace` token starting with `"    "` (4+ spaces) becomes an
indented code block - unless the very next token is a list marker,
which means this is sub-list indentation, not code.

### 2. Whitespace + list-marker probe

```rust
if matches!(self.peek_kind(), Some(TokenKind::Whitespace))
  && let Some(next) = self.tokens.get(self.pos + 1)
{
  match next.kind {
    TokenKind::UnorderedListItem => { self.advance(); return Some(self.parse_list(false, 0)); }
    TokenKind::OrderedListItem   => { self.advance(); return Some(self.parse_list(true,  0)); }
    _ => {},
  }
}
```

Top-level lists that begin with leading whitespace skip past the
whitespace and parse normally with `indent = 0`. (Lists encountered
deeper recurse via the inner `parse_list` loop with non-zero
`indent`.)

### Main match

```rust
match self.peek_kind()? {
  TokenKind::FrontmatterStart  => Some(self.parse_frontmatter()),
  TokenKind::Import            => Some(self.import_node()),
  TokenKind::Export            => Some(self.export_node()),
  TokenKind::Heading(_)        => Some(self.parse_heading()),
  TokenKind::CodeStart(n) if *n >= 3 => Some(self.parse_code_block()),
  TokenKind::JsxOpenTagStart   => Some(self.parse_jsx()),
  TokenKind::ExpressionStart   => Some(self.parse_jsx_expression()),
  TokenKind::MarkdownCommentStart => { self.skip_md_comment(); None },
  TokenKind::UnorderedListItem => Some(self.parse_list(false, 0)),
  TokenKind::OrderedListItem   => Some(self.parse_list(true,  0)),
  TokenKind::BlockQuote        => Some(self.parse_blockquote()),
  TokenKind::ThematicBreak     => { let s=self.current_span(); self.advance();
                                    Some(Node::HorizontalRule(HorizontalRule { span: s })) },
  TokenKind::HardBreak | TokenKind::SoftBreak => { self.advance(); None },
  _ => self.try_parse_table().or_else(|| Some(self.parse_paragraph())),
}
```

## Lists (`parse_list`)

Two parameters drive the loop:

- `ordered: bool` - which marker we accept.
- `indent: usize` - column the parent's marker sits at; nested
  recursive calls pass the deeper indent so the loop knows when a
  marker belongs to an outer list.

### Top-level vs nested

On the first iteration the cursor is already on the marker token (the
caller advanced past any indent). On subsequent iterations, when
`indent > 0`, the loop expects a `Whitespace` token of exactly
`indent` chars before the next marker:

```rust
if !first && indent > 0 {
  let aligned = matches!(
    self.peek(),
    Some(t) if matches!(t.kind, TokenKind::Whitespace) && t.raw.chars().count() == indent
  );
  if !aligned { break; }
  let next = self.tokens.get(self.pos + 1);
  let next_is_marker = matches!(
    next.map(|t| t.kind.clone()),
    Some(TokenKind::UnorderedListItem) | Some(TokenKind::OrderedListItem)
  );
  if !next_is_marker { break; }
  self.advance();
}
```

### Ordered marker fix-up

Ordered list markers like `3.` are emitted as
`OrderedListItem` followed by a `Text` token whose `raw` still
includes the trailing `.` (and trailing spaces). The loop strips
that:

```rust
if ordered {
  let is_text = matches!(self.peek_kind(), Some(TokenKind::Text));
  let raw_opt: Option<&'tokens str> = if is_text { self.peek_raw() } else { None };
  if let Some(raw) = raw_opt {
    let trimmed = raw.strip_prefix('.').unwrap_or(raw).trim_start_matches([' ', '\t']);
    if trimmed.is_empty() { self.advance(); }
    else if trimmed.len() != raw.len() {
      let pos = self.pos;
      self.tokens[pos].raw = trimmed;
    }
  }
}
```

### Nested + loose-paragraph continuation

After consuming an item's inline body, the loop scans forward while
the upcoming line is indented strictly deeper than `indent`:

```rust
while let Some(child_indent) = self.peek_leading_indent() {
  if child_indent <= indent { break; }
  let saved = self.pos;
  self.advance();
  match self.peek_kind() {
    Some(TokenKind::UnorderedListItem) => {
      let sub = self.parse_list(false, child_indent);
      Self::append_to_item(&mut item, sub);
    }
    Some(TokenKind::OrderedListItem) => {
      let sub = self.parse_list(true, child_indent);
      Self::append_to_item(&mut item, sub);
    }
    Some(_) => {
      // loose-list paragraph continuation
      let span = self.current_span();
      let inline = self.collect_inline_until_break();
      if inline.is_empty() { self.pos = saved; break; }
      Self::ensure_loose_item(&mut item, &span);
      Self::append_to_item(&mut item, Node::Paragraph(Paragraph { children: inline, span }));
    }
    None => { self.pos = saved; break; }
  }
  // eat the trailing break and loop
}
```

Three deeper-indent cases:

1. List marker -> recurse `parse_list(_, child_indent)`, attach as
   sub-list under the current item.
2. Plain content -> loose-list paragraph continuation: convert any
   raw inline body of the item into a `Paragraph` (via
   `ensure_loose_item`), then push the new run as another
   `Paragraph`.
3. Anything else -> rewind and exit; the parent dispatcher decides.

### Loose-list rewrite

After the per-item loop, if any item ended up with a `Paragraph`
child, every item gets normalised the same way:

```rust
let any_loose = items.iter().any(|n| match n {
  Node::ListItem(li) => li.children.iter().any(|c| matches!(c, Node::Paragraph(_))),
  Node::TaskListItem(t) => t.children.iter().any(|c| matches!(c, Node::Paragraph(_))),
  _ => false,
});
if any_loose {
  for n in items.iter_mut() { Self::ensure_loose_item(n, &span); }
}
```

### Task-list detection

`parse_one_list_item` sniffs for the GFM checkbox pattern after an
unordered marker:

```
WHITESPACE? Bracket Text(" "|"x"|"X") Bracket
```

Hit -> emit `TaskListItem { checked, children, span }` with
`checked = true` for `x`/`X`. Miss -> rewind and produce a regular
`ListItem`.

## Blockquote (`parse_blockquote`)

A depth-stack walker. Each iteration looks at one logical line.

```rust
let mut children:   Vec<Vec<Node>> = vec![Vec::new()];
let mut paragraphs: Vec<Vec<Node>> = vec![Vec::new()];
```

`children[i]` is the accumulated block-level contents at nesting
level `i+1`. `paragraphs[i]` is its in-progress inline run (paragraph
under construction).

### Per-line steps

1. `count_line_blockquote_markers` - count consecutive `>` markers
   for this line, skipping inter-marker whitespace.
2. `0` markers -> exit the outer loop.
3. Grow the stack: while `children.len() < line_markers` push empty
   vecs.
4. Shrink the stack: while `children.len() > line_markers` close the
   deepest level via `close_blockquote_level`.
5. Consume the markers (`consume_blockquote_markers`).
6. `collect_inline_until_break` for this line.
7. Examine the trailing break:
   - empty inline + pending paragraph -> flush pending into
     `children[top]`.
   - non-empty inline -> if `paragraphs[top]` already has content,
     push a single space `Text` then extend.
   - HardBreak after non-empty inline -> flush the paragraph
     (multiple paragraphs in one quote level).

### Multi-line merge

The "extend with a space separator" branch is the multi-line merge
fix: previously every line of `> a\n> b` produced its own paragraph;
now the depth-stack version merges them into one paragraph until a
HardBreak, matching CommonMark.

### Close

```rust
while children.len() > 1 {
  Self::close_blockquote_level(&mut children, &mut paragraphs, &para_span, &span);
}
```

Then flush the root level and wrap as `Node::Blockquote`.

`close_blockquote_level`:

```rust
fn close_blockquote_level(...) {
  let mut inner_children = children.pop().unwrap();
  let pending = paragraphs.pop().unwrap();
  if !pending.is_empty() {
    inner_children.push(Node::Paragraph(Paragraph { children: pending, span: para_span.clone() }));
  }
  let bq = Node::Blockquote(Blockquote { children: inner_children, span: bq_span.clone() });
  let parent_idx = children.len() - 1;
  children[parent_idx].push(bq);
}
```

The depth-stack rewrite replaced an older recursive approach that
mishandled lines like `>>>` (jumps in depth) and unbalanced
shrinking.

## Indented code block

`parse_indented_code` is invoked only when the dispatch pre-check
matches. Per iteration:

1. Require leading whitespace token starting with `"    "` (the first
   such line was already validated).
2. Push `t.raw[4..]` (the bit after the 4 indent spaces) into `buf`.
3. Append the rest of the tokens up to the next break verbatim.
4. Push `\n`.
5. On `SoftBreak`, peek the next token: if it's another 4+ space
   indent line, continue; otherwise rewind and exit.
6. On any other break (HardBreak, EOF), exit.

`lang` and `meta` are always `None` for indented code.

## Paragraph + setext

`parse_paragraph` is the fallback. After collecting inline content it
checks for a setext underline:

```rust
if matches!(self.peek_kind(), Some(TokenKind::SoftBreak)) {
  let saved = self.pos;
  self.advance();
  if let Some(lvl) = self.setext_underline_level() {
    self.eat_setext_underline();
    return Node::Heading(Heading { level: lvl, children, span });
  }
  self.pos = saved;
}
```

`setext_underline_level`:

- A run of `Eq` tokens followed by break/EOF -> `Some(1)`.
- A `ThematicBreak` whose `raw` is all `-` -> `Some(2)`.
- Else `None`.

`eat_setext_underline` consumes the matched underline tokens
(`Eq+` or one `ThematicBreak`).

## Fenced code block

`parse_code_block` runs when the lexer emits `CodeStart(n)` with `n >= 3`.

1. Remember the fence width `n`.
2. If the next token is `Text`, treat it as the info string.
3. Trim and split at first whitespace: `(lang, rest)`. Empty parts
   become `None`.
4. Concatenate every following token's `raw` into `value` until
   `CodeEnd(m)` with `m == n` is seen, or EOF.

## Heading (ATX)

```rust
let level = match self.peek_kind() {
  Some(TokenKind::Heading(n)) => *n,
  _ => 1,
};
self.advance();
let children = self.collect_inline_until_break();
Node::Heading(Heading { level, children, span })
```

## Frontmatter / Import / Export

Trivial wrappers. Frontmatter eats `FrontmatterStart`, optional
`FrontmatterContent`, optional `FrontmatterEnd` and stores the
content as `raw`. Import / Export wrap the upcoming token's `raw` in
the corresponding node.

## Recent fixes (relative to git history)

- Multi-line blockquote merge: paragraphs now extend with a single
  space across consecutive `>` lines until a HardBreak.
- Nested list handling: `parse_list(_, indent)` plus the
  `peek_leading_indent` deeper-line scan replaced an indent-agnostic
  loop that flattened sub-lists.
- Blockquote depth-stack: the `children` / `paragraphs` parallel
  stacks (with `close_blockquote_level`) replaced the old recursive
  walker, fixing jumps and unbalanced depths.
