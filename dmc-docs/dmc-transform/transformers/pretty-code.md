# PrettyCode

Replaces `Node::CodeBlock` with a syntect-highlighted `<figure>`
subtree. Replaces the JS chain `rehype-pretty-code` + `shiki`.

## Feature flag

`pretty-code` (default on). Pulls the `dmc-highlight` leaf crate.

## Input

Any `Node::CodeBlock { lang, meta, value }`. Skips blocks with
`lang == "mermaid"` (handled by the `Mermaid` pass).

## Output

```
<figure data-dmc-figure>
  [<figcaption data-dmc-title data-language="rust">lib.rs</figcaption>  // when title set
  ]
  <pre style="background-color:#1e1e2e;--dmc-light-bg:#eff1f5"
       data-language="rust"
       data-theme="dark:Catppuccin Mocha light:Catppuccin Latte">
    <code>
      <span data-line>
        <span style="color:#cba6f7;--dmc-light:#8839ef">fn</span>
        <span style="color:#cdd6f4;--dmc-light:#4c4f69"> </span>
        ...
      </span>
      <span data-line data-highlighted-line>
        ...
      </span>
    </code>
  </pre>
</figure>
```

`<figure>` always wraps. `<figcaption>` only present when `title=`
is set in meta.

## Config

```rust
pub struct PrettyCodeOptions {
    pub theme: PrettyCodeTheme,
    pub default_mode: Option<String>,
}

pub enum PrettyCodeTheme {
    Single(String),
    Multi(BTreeMap<String, String>),
}
```

Default = Multi `{ light: "Catppuccin Latte", dark: "Catppuccin Mocha" }`,
`default_mode = "dark"`.

```ts
prettyCode: {
  theme: { light: "Catppuccin Latte", dark: "Nord" },
  defaultMode: "dark",
}

// or:
prettyCode: { theme: "Catppuccin Mocha" }
```

`PrettyCodeTheme` is `#[serde(untagged)]`, so a string is parsed as
`Single` and an object as `Multi`.

## Multi-mode CSS variables

For each non-primary mode, every token style includes a CSS variable:

```html
<span style="color:#cba6f7;--dmc-light:#8839ef">fn</span>
```

Plus on `<pre>`:

```html
<pre style="background-color:#1e1e2e;--dmc-light-bg:#eff1f5">
```

Consumer CSS swaps modes:

```css
html.light pre,
html.light pre code,
html.light pre code span {
  color: var(--dmc-light);
}
html.light pre {
  background-color: var(--dmc-light-bg);
}
```

## Meta directives

Code-block meta string supports:

- `title="hello.rs"` -> `<figcaption>` with text.
- `{1,3-5}` -> add `data-highlighted-line` on lines 1, 3, 4, 5.

Both can co-occur:

````md
```rust title="lib.rs" {3,5}
fn main() {
    let x = 1;
    let y = 2;
    println!("{}", x);
    let z = 3;
}
```
````

## Single-tokenize multi-color

Calls `dmc_highlight::highlight_code_multi(code, lang, &theme_names)`.
One parse + scope walk; each theme contributes only color resolution.
Adjacent same-style tokens merged across all themes (matches shiki
coalescing).

## Example

Input:

````md
```rust title="lib.rs"
fn main() {}
```
````

After PrettyCode pass + `HtmlEmitter`:

```html
<figure data-dmc-figure>
  <figcaption data-dmc-title data-language="rust">lib.rs</figcaption>
  <pre data-language="rust" data-theme="dark:Catppuccin Mocha light:Catppuccin Latte"
       style="background-color:#1e1e2e;--dmc-light-bg:#eff1f5">
    <code>
      <span data-line>
        <span style="color:#cba6f7;--dmc-light:#8839ef">fn</span>
        <span style="color:#cdd6f4;--dmc-light:#4c4f69"> </span>
        <span style="color:#89b4fa;--dmc-light:#1e66f5">main</span>
        <span style="color:#9399b2;--dmc-light:#7c7f93">()</span>
        <span style="color:#cdd6f4;--dmc-light:#4c4f69"> </span>
        <span style="color:#9399b2;--dmc-light:#7c7f93">{}</span>
      </span>
    </code>
  </pre>
</figure>
```

## Plugin gate

When `pretty-code` feature is on, `rehype-pretty-code` and `shiki`
are stripped from the sidecar payload. So a config that lists only
those native-handles them at no cost.

## Trade-off vs shiki

| | dmc PrettyCode | rehype-pretty-code |
|-|----------------|--------------------|
| engine | syntect (Rust) | shiki (JS) |
| speed | ~150 us per block | ~500 us - 2 ms per block |
| theme coverage | ~25 bundled (bat themes) | shipped shiki themes |
| grammar coverage | ~250 bundled | shipped shiki grammars |
| visual parity | very close | reference |
| span count | usually +5-10% (cosmetic) | reference |
