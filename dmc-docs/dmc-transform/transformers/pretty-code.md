# `pretty-code`

Pre-rendered syntax highlighter. Replaces fenced ``` ``` ``` blocks with
a velite-shaped `<pre><code>...</code></pre>` tree: each token becomes a
`<span style="color:#xxxxxx">`, each line gets `<span class="line">`,
and lines listed in `{1,3-5}` meta receive a configurable highlight
attribute.

- **Source:** `dmc-transform/src/builtin/pretty_code.rs`
- **Feature flag:** `pretty-code`
- **Config struct:** [`PrettyCodeOptions`](../src/config.rs)
- **TS slot:** `markdown.prettyCode` / `mdx.prettyCode`

## Output JSX

```text
<div data-rehype-pretty-code-fragment="">
  <figcaption data-rehype-pretty-code-title data-language="...">filename</figcaption>
  <pre __rawString__="..." data-language="..." data-theme="<mode>">
    <code data-language="..." data-theme="<mode>">
      <span class="line">
        <span style="color:#XXX">token</span>
        ...
      </span>
      <span class="line" data-highlighted-line="">...</span>
    </code>
  </pre>
  <pre data-theme="<other>">...</pre>   <!-- one per theme -->
</div>
```

Single-theme: one `<pre>` (`data-theme=""`). Multi-theme: one
`<pre data-theme="<mode>">` per entry; consumer CSS shows the active
mode by toggling visibility.

## Full configuration

```ts
import { defineConfig } from '@gentleduck/md/config'

export default defineConfig({
  markdown: {
    prettyCode: {
      // 1) THEME - pick one form
      theme: 'Catppuccin Mocha',                              // single
      // theme: { light: 'Catppuccin Latte', dark: 'Catppuccin Mocha' }, // multi
      defaultMode: 'dark',                                    // unprefixed color/bg source

      // 2) DOM SHAPE
      fragmentWrapper: true,                                  // <div data-rehype-pretty-code-fragment="">
      keepRawString: true,                                    // <pre __rawString__="..."> for Copy button
      lineClass: 'line',                                      // class on per-line span
      highlightedLineAttr: 'data-highlighted-line',           // attr on `{1,3-5}` lines
      renderTitle: true,                                      // <figcaption> from title="..." meta
      includeDataLanguage: true,                              // data-language on <pre>+<code>

      // 3) LANGUAGE BEHAVIOR
      defaultLanguage: 'plaintext',                           // fences without lang
      fallbackToPlaintext: true,                              // unknown langs -> plaintext
      skipLanguages: ['mermaid', 'math', 'd2'],               // pass these through unchanged

      // 4) WHITESPACE
      tabSize: 2,                                             // expand tabs before highlighting
    },
  },
})
```

## Knob reference

| Knob | Default | Effect |
|---|---|---|
| `theme` | `{ light: "Catppuccin Latte", dark: "Catppuccin Mocha" }` | Single bundled theme name, OR `{ mode: theme }` map for multi-mode. |
| `defaultMode` | `"dark"` if present, else first key | Mode whose colors fill unprefixed `color` / `background-color`. |
| `keepRawString` | `true` | Sets `__rawString__` on each `<pre>` so consumer's `<PreBlock>` can render a Copy button. |
| `fragmentWrapper` | `true` | Wraps panes in `<div data-rehype-pretty-code-fragment="">`. Off -> bare `<div>` (still single-rooted). |
| `lineClass` | `"line"` | Class on the per-line `<span>`. |
| `highlightedLineAttr` | `"data-highlighted-line"` | Attribute set on lines listed in `{1,3-5}` meta. |
| `defaultLanguage` | `"plaintext"` | Used when fence has no language. |
| `fallbackToPlaintext` | `true` | Unknown langs -> plaintext. Off -> block left as raw `CodeBlock`. |
| `renderTitle` | `true` | Emit `<figcaption data-rehype-pretty-code-title>` from `title="..."` meta. |
| `includeDataLanguage` | `true` | Include `data-language` attr on `<pre>` and `<code>`. |
| `skipLanguages` | `[]` | Languages to pass through unchanged. `mermaid` is always skipped (owned by the mermaid transformer). |
| `tabSize` | unset | Expand tab characters to N spaces before highlighting. |

## Meta syntax

Fence info string accepts:

- `title="path/to/file.rs"` -> `<figcaption>`
- `{1,3-5}` -> highlighted lines (range and comma-separated)

Example:

````text
```rust title="hello.rs" {2,4-6}
fn main() {
    println!("hi");
    let x = 1;
    let y = 2;
    let z = 3;
    let sum = x + y + z;
}
```
````

## Themes

Themes come from the bundled syntect theme set
(`dmc-highlight::SyntaxBundle`). Common bundled names: `Catppuccin Latte`,
`Catppuccin Mocha`, `Nord`, `One Dark`, `Solarized Light`, `Solarized Dark`,
`InspiredGitHub`, `base16-ocean.dark`, `base16-eighties.dark`.

## Sidecar opt-out

Add `"rehype-pretty-code"` or `"shiki"` to `markdown.preferSidecar` to
drop the native highlighter and route through the JS sidecar.
