<p align="center">
  <img src="../../public/logo-dark.svg" alt="dmc samples" width="120"/>
</p>

<h1 align="center">dmc samples</h1>

<p align="center">
  Hand-picked MDX fixtures used across examples, parser tests, and bench harness.
</p>

<p align="center">
  <a href="../../LICENSE">MIT</a> -
  <a href="../../README.md">repo</a>
</p>

---

## Files

| file | purpose |
| --- | --- |
| `architecture.mdx` | dmc pipeline diagram |
| `system-design.mdx` | end-to-end flow with mermaid |
| `headings.mdx` | heading + autolink coverage |
| `mermaid.mdx` | mermaid block samples |
| `bare-urls.mdx` | autolink edge cases |
| `code-import.mdx` | `<CodeImport>` transformer |
| `npm-commands.mdx` | `<NpmCommand>` tabs |
| `index.mdx` | top-level navigation |
| `errors/*.mdx` | malformed inputs the diagnostic engine should flag |
| `snippets/` | smaller fragments referenced by tests |

## Used by

- `examples/nextjs/` runtime renderer
- `examples/web/` runtime renderer
- `dmc-parser/tests/*.rs` snapshot tests
- `dmc-core/examples/bench.rs` corpus

Modify with care: tests pin against snapshot output.
