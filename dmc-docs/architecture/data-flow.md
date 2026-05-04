# Data flow

What every byte goes through, from source on disk to JSON record.

## Read

```mermaid
flowchart LR
    Disk[posts/hello.mdx] --> Read[std::fs::read_to_string]
    Read --> Source[String]
```

## Preprocess

```mermaid
flowchart LR
    Source --> Math[Math::preprocess_source]
    Math --> Pre[String with $...$ -> MathMl JSX]
```

`$...$` and `$$...$$` are rewritten to `<MathMl mathml="..."/>` JSX
before the lexer so `_`/`^` inside math are not interpreted as
emphasis markers.

## Lex

```mermaid
flowchart LR
    Pre --> Lexer[Lexer::scan_tokens]
    Lexer --> Tokens[Vec<Token>]
```

Tokens carry `kind` + `span` + raw `&str`. Whitespace tokens are
preserved (needed for inline spacing around links).

## Parse

```mermaid
flowchart LR
    Tokens --> Parser[Parser::parse]
    Parser --> Doc[Document]
```

Document is `{ children: Vec<Node>, span }` where `Node` is the AST
enum (Heading, Paragraph, CodeBlock, JsxElement, ...).

## Transform

```mermaid
flowchart LR
    Doc --> Pipe[Pipeline::with_defaults_for cfg]
    Pipe --> CI[CodeImport]
    CI --> BU[BareUrlAutolink]
    BU --> AH[AutolinkHeadings]
    AH --> NPM[NpmCommand]
    NPM --> Mer[Mermaid]
    Mer --> Em[Emoji]
    Em --> M[Math]
    M --> PC[PrettyCode]
    PC --> CL[CopyLinkedFiles]
    CL --> Doc2[Document mut]
```

Order matters: e.g. `BareUrlAutolink` runs before `AutolinkHeadings`;
`Math` runs before `PrettyCode` so math nodes are not seen by the
syntax highlighter. `with_defaults_for` controls the order.

## Walk + emit

```mermaid
flowchart TD
    Doc2 --> Walker[Walker pre-order DFS]
    Walker --> Acc[Accumulator: frontmatter, toc, excerpt, metadata]
    Walker --> Html[HtmlEmitter: SSR HTML string]
    Walker --> Body[MdxBodyEmitter: JS body for runtime MDX]
    Acc & Html & Body --> Out[CompileOutput]
```

One DFS, three sinks. Each sink sees every node. Sinks fire `enter`
slice-order, `leave` LIFO so structural close logic mirrors push.

## Sidecar (optional)

```mermaid
flowchart LR
    Out -- if has_js_plugins --> Pool[Sidecar pool]
    Pool --> Node[NDJSON request]
    Node --> Pool
    Pool -- replace html --> Out2[CompileOutput]
```

When the user listed unified plugins not owned by native transformers,
the sidecar receives `compiled.content` and returns rendered HTML that
replaces `compiled.html`. The plugin gate strips native-owned names
before dispatch.

## Schema validate

```mermaid
flowchart LR
    Out2 --> Schema[compile_descriptor + parse]
    Schema --> Validated[fm Value]
```

Frontmatter validated against the collection's schema. Failures emit
a diagnostic; record falls back to raw frontmatter.

## Cache + write

```mermaid
flowchart LR
    Validated --> Rec[build_velite_record]
    Rec --> Cache[FileCache.put]
    Rec --> JSON[serde_json::to_string_pretty]
    JSON --> Disk2[output/name.json]
```

## Index

After every collection finishes:

```mermaid
flowchart LR
    Coll[output/*.json] --> WI[index::write_index]
    WI --> JS[output/index.js]
    WI --> DTS[output/index.d.ts]
```

Re-exports each `<name>.json`. Type aliases reference
`typeof import(config)["collections"]` so user types flow through.
