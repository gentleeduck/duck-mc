---
"@gentleduck/md": patch
---

Mermaid diagrams render in one long-lived headless browser instead of a fresh `mmdc` (node and Chrome) for every diagram and theme. On a 548-page docs site with 274 diagrams (558 renders), a cold build went from 278s to 89s.

The SVGs are the bytes `mmdc` writes: the renderer runs mermaid-cli's own `renderMermaid` with the options `mmdc` passes, so existing `outputDir` caches stay valid. It loads the `@mermaid-js/mermaid-cli` package behind the `mmdc` on `PATH`; where there is none, such as an `mmdc` that wraps the docker image, each diagram runs `mmdc` as before. The browser closes 30s after its last diagram, or with the process.
