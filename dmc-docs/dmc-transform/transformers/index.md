# Transformers

Every built-in transformer in `dmc-transform`, indexed.

| transformer | feature | replaces |
|-------------|---------|----------|
| [`code-import`](code-import.md) | always on | reading `file=` attrs in code blocks |
| [`bare-url`](bare-url.md) | always on | bare URL autolinking (gfm style) |
| [`autolink-headings`](autolink-headings.md) | always on | `rehype-slug` + `rehype-autolink-headings` |
| [`disable-gfm`](disable-gfm.md) | always on (config-gated) | strict CommonMark mode |
| [`npm-command`](npm-command.md) | `npm-command` | manual package-manager tab markup |
| [`mermaid`](mermaid.md) | `mermaid` | renders mermaid via `mmdc` |
| [`emoji`](emoji.md) | `emoji` | `remark-emoji` |
| [`math`](math.md) | `math` | `remark-math` + `rehype-katex` (or MathML) |
| [`pretty-code`](pretty-code.md) | `pretty-code` | `rehype-pretty-code` + `shiki` |
| [`copy-linked-files`](copy-linked-files.md) | `assets` (config-gated) | webpack-style asset copy |

## Order in the default pipeline

```
CodeImport
BareUrlAutolink
AutolinkHeadings
DisableGfm        (only when markdown_gfm: false)
NpmCommand        (feature: npm-command)
Mermaid           (feature: mermaid)
Emoji             (feature: emoji)
Math              (feature: math)
PrettyCode        (feature: pretty-code)
CopyLinkedFiles   (feature: assets, config-gated)
```

See [`../pipeline.md`](../pipeline.md) for the rationale.

## Quick navigation

- Building one yourself: [`../writing-a-transformer.md`](../writing-a-transformer.md)
- Visitor pattern: [`../visitor.md`](../visitor.md)
- Feature flags: [`../feature-flags.md`](../feature-flags.md)
- API surface: [`../api.md`](../api.md)
