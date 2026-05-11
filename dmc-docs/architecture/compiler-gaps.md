# dmc compiler - gaps surfaced by the duck-ui-style example

> Historical note: `examples/nextjs-dmc-full` has since been removed.
> Part 2 below (and the `bun install` / `bun run content` commands that
> reference it) is kept as a record of what that run found; it is no
> longer reproducible as written. Part 1 (`examples/nextjs` vs
> `examples/nextjs-velite`) still runs.

Side-by-side runs:

| dir | runner | source |
| --- | --- | --- |
| `examples/nextjs` | dmc native pipeline | `content/docs/{hello,kitchen-sink}.mdx` |
| `examples/nextjs-velite` | velite (remark/rehype JS plugins) | same MDX, separate copy |
| `examples/nextjs-dmc-full` (removed) | dmc native, duck-ui-style config | `content/docs/duck-{ui,hooks}/**` |

Run `bun run content` in each. dmc binaries live at
`dmc-napi/dmc.linux-x64-gnu.node`; the workspace examples reach it via the
`@gentleduck/md` package.

## Part 1 - dmc vs velite on the kitchen-sink MDX

Same source, two compile chains.

### Output shape

| feature | dmc | velite |
| --- | --- | --- |
| record fields | `body`, `content`, `contentType`, `description`, `excerpt`, `flattenedPath`, `html`, `metadata`, `permalink`, `slug`, `sourceFileDir`, `sourceFileName`, `sourceFilePath`, `tags`, `title`, `toc` | only schema-declared (`description`, `html`, `permalink`, `slug`, `tags`, `title`) |
| code-block `<figure>` attr | `data-dmc-figure` | `data-rehype-pretty-code-figure` |
| code-block `<figcaption>` attr | `data-dmc-title` | `data-rehype-pretty-code-title` |
| `<pre>` per code block | one (dual theme via custom props) | two (light + dark) |
| `<pre>` style | `background-color:#1e1e2e;--dmc-light-bg:#eff1f5` | `--shiki-light:#...;--shiki-dark:#...;--shiki-light-bg:#...;--shiki-dark-bg:#...` |
| token span style | `color:#cdd6f4;--dmc-light:#4c4f69` | `--shiki-light:#...;--shiki-dark:#...` |
| `<span data-line>` | `data-line` (boolean attr, no value) | `data-line=""` (empty string) |
| heading anchor | `<a href="#..." title="Link to section">` | `<a href="#...">` (no title) |
| heading anchor link text | leading space (`<a> Inline marks</a>`) | tight (`<a>Inline marks</a>`) |
| task-list `<ul>` class | none | `class="contains-task-list"` (rehype-task-list) |
| void-tag close style | `<input ... />`, `<img ... />` (XHTML self-close) | `<input ...>`, `<img ...>` (HTML5) |
| math | inline `<math>...</math>` (browser MathML) | `<span class="katex">...</span>` (KaTeX HTML; needs CDN CSS) |

Tag counts otherwise match (h1/h2/h3, p, code, ul/ol/li, table/tr/td, blockquote,
strong/em/del, hr, figure, pre, math/mrow/mi/mo/msup/mn). Span counts: dmc
710 vs velite 672 - dmc emits an extra wrapping span per highlighted line
because each token carries its own `--dmc-light` custom property, while
shiki coalesces same-color runs.

### `data-theme` value

- dmc: `dark:Catppuccin Mocha light:Catppuccin Latte` (single string,
  custom property-driven theme switch)
- velite: `catppuccin-latte catppuccin-mocha` (space-separated; one `<pre>`
  per theme is rendered with a sibling block, CSS toggles by `[data-theme]`)

### Same MDX, identical structure summary

For prose, headings, lists, tables (without inline code), images, blockquotes,
hr, GFM strikethrough, autolinks, and emoji, the two pipelines produce
**identical tag counts and identical text content**. The visible deltas are
all in the code-highlighting envelope, the math envelope, and trivial
HTML-vs-XHTML self-close style.

## Part 2 - dmc on a duck-ui-style example (`nextjs-dmc-full`)

Goal: stress the surfaces real component-library docs depend on.

What was exercised:

- multi-line JSX attribute lists (`<ComponentPreview\n  name="..."\n  />`)
- block JSX with markdown children (`<Tabs>\n\n<TabsList>...</TabsList>\n\n...\n</Tabs>`)
- backtick template literal in JSX expression
  (`<MermaidDiagram chart={\`graph TD\n...\n\`} />`)
- nested JSX in attribute value (`icon={<Zap />}`)
- compound components (`<Steps><Step>...</Step></Steps>`)
- per-package collections (`duckUi`, `duckHooks`)
- `s.transform()` projection on `s.object()` schema

What now compiles cleanly (was broken before this round):

- multi-line JSX attribute lists. The lexer's `lex_jsx_tag` now skips
  `\n` between attrs (new `skip_jsx_tag_ws`).
- backtick template literals in JSX expressions. Both
  `lex_jsx_attr_expr` and `lex_expression` now track an in-quote /
  in-template state, so `}` and `\n` inside `` `...` ``, `"..."`, `'...'`
  do not prematurely close the expression.
- nested JSX inside an attribute value. dmc-codegen's `jsx_props` now
  re-runs `dmc_parser::parse_inline_str` on any expression whose
  trimmed source begins with `<`, then routes the resulting node
  through the existing `inline_expr` emitter.

### Compiler gaps still observed

1. **Indented JSX child wrapped in `<p>`**.
   Source:
   ```mdx
   <TabsList>
     <TabsTrigger value="cli">CLI</TabsTrigger>
     <TabsTrigger value="manual">Manual</TabsTrigger>
   </TabsList>
   ```
   Emitted:
   ```js
   jsxs(TabsList, { children: [
     jsxs("p", { children: ["  ", jsxs(TabsTrigger, ...) ] }), // <- extra <p>
     jsxs("p", { children: ["  ", jsxs(TabsTrigger, ...) ] }),
   ]})
   ```
   The leading two spaces on each child line are kept as a Text node and
   the parser's block path wraps the whole inline run in a `<p>`. Velite
   / MDX2 strip leading whitespace inside JSX children and place the
   inner element directly. Fix needed in `parser/jsx.rs::parse_jsx`
   children loop: if a JSX element is the only non-whitespace token on
   a line, do not wrap it in a paragraph.

2. **`pm-tabs` transformer is overzealous**.
   Source:
   ```mdx
   <TabsContent value="cli">

   ```bash
   npx @gentleduck/cli add accordion
   ```

   </TabsContent>
   ```
   Emitted: a `<PackageManagerTabs npm yarn pnpm bun />` with synthesized
   commands:
   ```
   yarn: "yarn run @gentleduck/cli add accordion"
   pnpm: "pnpm run @gentleduck/cli add accordion"
   bun:  "bunx   @gentleduck/cli add accordion"
   ```
   `yarn run` / `pnpm run` are not equivalent to `npx`. The native
   `pm-tabs` transformer should either:
   (a) only fire for `npm install ...` / `npm i ...` style commands, not
       arbitrary `npx` invocations, or
   (b) translate `npx` -> `yarn dlx` / `pnpm dlx` / `bunx`, not `... run`.
   File: `dmc-transform/src/pm_tabs.rs` (or wherever `PackageManagerTabs`
   synthesis lives).

3. **Backtick-escaped pipe in table cells splits columns**.
   Source:
   ```md
   | Prop | Type | Default |
   | --- | --- | --- |
   | `type` | `"single" \| "multiple"` | `"single"` |
   ```
   Emitted:
   ```js
   jsxs("tr", { children: [
     jsxs("td", { children: [jsx("code", { children: "type" })] }),
     jsxs("td", { children: [jsx("code", { children: "\"single\" \\" })] }),
     jsxs("td", { children: ["\"multiple\"", jsx("code", { children: "" })] }),
     // <- columns 3+4+5 are now misaligned; "\"single\"" was supposed to be
     //   a single Type cell containing "single" \| "multiple"
   ]})
   ```
   The cell tokenizer splits on `|` regardless of whether the `|` is
   inside backticks or escaped with `\`. Fix: track in-code-span and
   honor `\|` as a literal column character (GFM-spec behavior).
   File: `dmc-parser/src/table.rs`.

4. **Square brackets inside table cells get parsed as link syntax**.
   Source: `| `string \| string[]` | `-` |`
   Emitted contains `jsxs("a", { href: "", children: [""] })` - an
   empty `<a>`. The cell-inline parser ran the link-detector across
   the cell text and matched `string[]` as a link reference with no
   target. Fix: the `[` / `]` link path should require a following
   `(...)` or `[...]` reference; bare `[...]` should remain text. Likely
   already correct in the inline parser but the table cell path uses
   a slightly different inline path that misfires here.
   File: `dmc-parser/src/inline.rs` (link rule reused by table cells).

5. **`<` followed by uppercase inside a code span string still emits as
   raw JSX in some edge cases**.
   Source: `React.HTMLProps<HTMLDivElement>` inside an inline `\``-fenced
   span renders fine - the inline-code path treats it as text and the
   codegen wraps it in `js_string`. **No regression here**, but worth a
   regression test: `<Btn` immediately after `\``...`\`` with no whitespace
   has historically been a confusion point.

6. **Schema strictness diverges from velite**.
   velite's `s.object({...}).transform(...)` returns *only* the fields you
   declared (plus what `transform` adds). dmc returns those fields plus
   the engine's default fields (`body`, `content`, `contentType`,
   `excerpt`, `flattenedPath`, `html`, `metadata`, `sourceFileDir`,
   `sourceFileName`, `sourceFilePath`, `toc`). Either intentional - dmc
   surfaces more by default - or a strict-mode flag should opt
   out of the engine default fields when the user has supplied a
   schema. File: `dmc-core/src/collection.rs`.

7. **Heading anchor leading space**.
   dmc emits `<h2 id="x"><a href="#x" title="Link to section"> X</a></h2>`.
   The leading space inside the anchor text is from the autolink-headings
   transformer prepending a `" "` text node before the heading text. velite
   (rehype-autolink-headings with `behavior: "wrap"`) emits no leading
   space. Cosmetic, but wraps the visible heading with a thin gap.
   File: `dmc-transform/src/autolink_headings.rs`.

### Things that work cleanly (verified)

- multi-line JSX attribute lists -> attrs round-trip as expected.
- backtick template literal in JSX expression -> preserved verbatim.
- nested JSX in attribute value -> compiled.
- multi-line block JSX `<Tabs>...</Tabs>` with embedded markdown
  (headings, code fences, paragraphs) -> children are real AST nodes
  and the wrapping JSX call is shaped correctly.
- per-package collections + `s.transform` -> both `DuckUi.json` and
  `DuckHooks.json` emit, with `slug` / `permalink` derived from the
  source path via the transform.
- frontmatter typing reaches the emitted `index.d.ts` via
  `Collections['duckUi']['schema']['_output']` so consumers can do
  `type Doc = Collections['duckUi']['schema']['_output']`.

## Reproducing

```sh
# build the napi binary
bun run --cwd dmc-napi build

# kitchen-sink baseline
(cd examples/nextjs && bun run content)
(cd examples/nextjs-velite && bun run content)

# duck-ui-style stress test
(cd examples/nextjs-dmc-full && bun install && bun run content)

# diff the two kitchen-sink HTMLs
jq -r '.[] | select(.title=="Kitchen Sink") | .html' \
  examples/nextjs/.gentleduck/doc.json > /tmp/dmc.html
jq -r '.[] | select(.title=="Kitchen Sink") | .html' \
  examples/nextjs-velite/.velite/docs.json > /tmp/velite.html
diff -u /tmp/dmc.html /tmp/velite.html | less
```
