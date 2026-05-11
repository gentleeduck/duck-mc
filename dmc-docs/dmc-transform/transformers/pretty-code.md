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
| `classed` | `false` | Emit class-based `<span class="dmc-<hex>...">` tokens once + write `dmc.<mode>.css` per theme, instead of per-token inline styles. See "Class-based output" below. |

## Class-based output (`classed`)

By default every syntax token gets an inline `<span style="color:#hex">`,
and with a `light` + `dark` theme map each code block is rendered twice
(one `<pre>` per theme). For a large docs set that inline hex on every
token - duplicated per theme - is the bulk of the compiled output.

Set `classed: true` to switch to a class-based scheme:

- Tokens are rendered ONCE as `<span class="dmc-<hex>...">`. The class
  name is a color tuple: one lowercase 6-digit hex foreground per
  configured theme, joined by `-`, in canonical theme order (light,
  dark, then any other modes alphabetically) - e.g. `dmc-89b4fa-8839ef`
  for a `light`+`dark` map, or `dmc-89b4fa` for a single theme. A
  font-style suffix is appended when the default-mode token is bold /
  italic / underline: `-b`, `-i`, `-u`, or a combo like `-bi` (order
  b, i, u). Tokens whose foreground equals every theme's default
  foreground (and have no font style) get no class and inherit from the
  `<pre>`. The `<pre>` carries `class="dmc-pre"`. There is no per-theme
  `<pre>` duplication and no inline-style objects in the MDX body, so
  `className: "dmc-..."` beats `style: { color: "#..." }` outright.
- At the end of the build, one stylesheet per configured theme is
  written to the output data dir: `dmc.<mode>.css` for each `mode ->
  theme` entry in a multi-theme map (e.g. `dmc.dark.css`,
  `dmc.light.css`), or a single `dmc.css` for a single unnamed theme.
  Each rule in a per-mode file is scoped under `[data-theme="<mode>"] `
  (a root `[data-theme="<mode>"] .dmc-pre { color; background-color }`
  rule plus one `[data-theme="<mode>"] .dmc-<class> { color; ... }` rule
  per recorded token class), so the consumer loads every theme CSS file
  and an ancestor `[data-theme]` attribute selects which colors apply.
  The single-theme `dmc.css` has bare, unscoped rules.
- Size: the output shrinks a lot. The token class names are short and
  repeat across the whole corpus (they coalesce exactly like the old
  color-based runs), the highlight pass runs once instead of once per
  theme, and the colors live in a handful of small shared CSS files
  instead of an inline hex string on every token in every page.
- `includePreBackground: false` drops the `background-color` from the
  `.dmc-pre` root rule (the foreground stays) so consumer chrome can
  own the surface color.

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
