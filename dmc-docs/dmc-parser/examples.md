# dmc-parser examples

## GFM table

```rust
use dmc_parser::{parse, ast::*};

let doc = parse("\
| h1 | h2 |
|----|----|
| a  | b  |
");

let table = match &doc.children[0] {
    Node::Table(t) => t,
    _ => panic!("expected table"),
};
assert_eq!(table.children.len(), 2);              // header + 1 row
assert_eq!(table.children[0].cells.len(), 2);
```

## Nested list with mixed kinds

```rust
use dmc_parser::{parse, ast::*};

let doc = parse("\
1. ordered head
   - bullet child
   - another bullet
     1. ordered grandchild
2. ordered next
");

let list = match &doc.children[0] {
    Node::List(l) if l.ordered => l,
    _ => panic!("expected ordered list"),
};
assert_eq!(list.children.len(), 2);                       // ordered head, ordered next
let first = match &list.children[0] {
    Node::ListItem(li) => li,
    _ => unreachable!(),
};
let inner = first.children.iter().find_map(|n| match n {
    Node::List(l) if !l.ordered => Some(l),
    _ => None,
}).unwrap();
assert_eq!(inner.children.len(), 2);                      // bullet child, another bullet
```

## Math + inline mixed paragraph

```rust
use dmc_parser::{parse, ast::*};

let doc = parse("\
This mixes `code`, **bold**, an emoji :sparkles:, and a [link](https://x).
");

let para = match &doc.children[0] {
    Node::Paragraph(p) => p,
    _ => panic!(),
};
let kinds: Vec<_> = para.children.iter().map(|n| match n {
    Node::Text(_) => "text",
    Node::InlineCode(_) => "code",
    Node::Bold(_) => "bold",
    Node::Link(_) => "link",
    _ => "other",
}).collect();

assert!(kinds.contains(&"text"));
assert!(kinds.contains(&"code"));
assert!(kinds.contains(&"bold"));
assert!(kinds.contains(&"link"));
```

## Nested blockquotes

```rust
use dmc_parser::{parse, ast::*};

let doc = parse("\
> outer
>
> > inner
> >
> > > deepest
");

let bq = match &doc.children[0] {
    Node::Blockquote(b) => b,
    _ => panic!(),
};
// outer: <p>outer</p> + nested blockquote
let nested = bq.children.iter().find_map(|n| match n {
    Node::Blockquote(b) => Some(b),
    _ => None,
}).unwrap();
// nested: <p>inner</p> + nested-nested
assert!(nested.children.iter().any(|n| matches!(n, Node::Blockquote(_))));
```

Stack-based depth tracking keeps nesting accurate. See
`dmc-docs/dmc-parser/block-parser.md`.

## Inline parsing on a free string

```rust
use dmc_parser::parse_inline_str;
use dmc_parser::ast::Node;

let nodes = parse_inline_str("**bold** and *italic* and `code`");

let kinds: Vec<&str> = nodes.iter().map(|n| match n {
    Node::Text(_) => "text",
    Node::Bold(_) => "bold",
    Node::Italic(_) => "italic",
    Node::InlineCode(_) => "code",
    _ => "other",
}).collect();

assert!(kinds.contains(&"bold"));
assert!(kinds.contains(&"italic"));
assert!(kinds.contains(&"code"));
```

Used by table cells.

## With diagnostics

```rust
use std::sync::Arc;
use dmc_diagnostic::{Code, metadata::{Origin, SourceMeta}};
use dmc_lexer::Lexer;
use dmc_parser::Parser;
use duck_diagnostic::DiagnosticEngine;

let source = "[broken";
let meta = Arc::from(SourceMeta {
    path: Arc::from("<inline>"),
    version: 0,
    origin: Origin::Inline("<inline>"),
});

let mut lex_engine = DiagnosticEngine::<Code>::new();
let mut lexer = Lexer::new(source, meta.clone(), &mut lex_engine);
let _ = lexer.scan_tokens();
let tokens = std::mem::take(&mut lexer.tokens);
drop(lexer);

let mut parse_engine = DiagnosticEngine::<Code>::new();
let mut parser = Parser::new(tokens, meta, &mut parse_engine);
let _doc = parser.parse();

for d in parse_engine.iter() {
    println!("[{}] {}", d.code.code(), d.message);
}
```

Lexer engine and parse engine usually share a single `DiagnosticEngine`
in production paths (`Compiler::compile_with_pipeline`).
