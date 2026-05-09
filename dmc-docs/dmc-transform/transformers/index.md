# Transformers

Built-in AST passes shipped with `dmc-transform`. Each is a single-purpose
transformer that mutates the parsed [`Document`] in place. The
[`Pipeline::with_defaults_for`] builder registers them in a fixed order
(see `pipeline.md`).

## Catalogue

| Doc | Transformer | Default | Configurable | Sidecar names |
|---|---|---|---|---|
| [`assign-heading-ids`](./assign-heading-ids.md) | `AssignHeadingIds` | always on | — | — |
| [`autolink-headings`](./autolink-headings.md) | `AutolinkHeadings` | on | yes | `rehype-slug`, `rehype-autolink-headings` |
| [`bare-url`](./bare-url.md) | `BareUrlAutolink` | always on | — | — |
| [`code-import`](./code-import.md) | `CodeImport` | always on | — | — |
| [`component-preview`](./component-preview.md) | `ComponentPreview` | on (consumer-driven) | yes | — |
| [`component-source`](./component-source.md) | `ComponentSource` | on (consumer-driven) | yes | — |
| [`copy-linked-files`](./copy-linked-files.md) | `CopyLinkedFiles` | off | yes | — |
| [`disable-gfm`](./disable-gfm.md) | `DisableGfm` | off | `markdown.gfm: false` | `remark-gfm` |
| [`emoji`](./emoji.md) | `Emoji` | feature-gated | on/off | `remark-emoji` |
| [`math`](./math.md) | `Math` | feature-gated | on/off + engine | `remark-math`, `rehype-katex`, `rehype-mathjax` |
| [`mermaid`](./mermaid.md) | `Mermaid` | feature-gated | full pass-through | `mermaid`, `rehype-mermaid`, `remark-mermaid` |
| [`npm-command`](./npm-command.md) | `NpmCommand` | feature-gated | — | — |
| [`pretty-code`](./pretty-code.md) | `PrettyCode` | feature-gated | full DOM-shape control | `rehype-pretty-code`, `shiki` |

## Configuration surface

Per-transformer config types live in [`crate::config`]:

- [`PrettyCodeOptions`] / [`PrettyCodeTheme`]
- [`MermaidOptions`] / [`MermaidThemeMode`]
- [`MathEngine`]
- [`CopyLinkedFilesOptions`]

The TS side mirrors them in `@gentleduck/md`'s `MarkdownOptions` /
`MdxOptions` slots.

## Sidecar opt-out

Native transformers are dropped automatically when the user lists the
matching JS plugin in `markdown.preferSidecar` (or sets
`markdown.forceSidecar: true`). Mapping in
[`CompileConfig::pipeline_config`].
