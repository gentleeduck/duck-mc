# dmc vs velite - side-by-side Next.js demos

Two Next.js apps render the same MDX through different compile chains.
Open both in browser tabs and compare.

## Layout

```text
examples/
|- nextjs/              dmc - native Rust pipeline (gfm + pretty-code + math + emoji + slug + autolink)
|- nextjs-velite/       velite - JS plugin chain (remark-gfm + remark-math + rehype-katex + remark-emoji + rehype-pretty-code + rehype-slug + rehype-autolink-headings)
`- nextjs/content/docs/ shared MDX fixtures (kitchen-sink.mdx + hello.mdx)
```

The velite app keeps its own copy of the MDX under
`nextjs-velite/content/docs/` to avoid path-aware coupling. Edit both,
copy between them, or symlink them if you want strict parity during a demo.

## Run them

From the repo root:

```bash
# build the napi binary once (rust -> .node)
pnpm --filter @gentleduck/md run build

# dmc app on http://localhost:3000
pnpm --filter dmc-nextjs dev

# velite app on http://localhost:3001 (in a second terminal)
pnpm --filter velite-nextjs dev
```

Both apps:

- prebuild content via `pnpm content` (dmc: `tsx scripts/build-content.ts`,
  velite: `velite build`)
- emit a JSON dataset under `.gentleduck/` (dmc) or `.velite/` (velite)
- the slug page reads `doc.html` and `dangerouslySetInnerHTML`s it

## What to look for

Visit `/docs/kitchen-sink` on both ports and compare these features:

| feature | dmc rendering | velite rendering |
| --- | --- | --- |
| code blocks (`pretty-code`) | `<pre><code><span data-line>` with inline `color:#xxx` + `--shiki-light:#yyy` CSS vars (multi-theme) | same shape via `rehype-pretty-code` + shiki |
| math `$x^2$` / `$$\int$$` | `<math display="...">` MathML rendered natively by the browser | KaTeX-injected `<span class="katex">` HTML |
| emoji `:rocket:` | unicode emoji in text | unicode emoji in text (matches) |
| GFM tables / task lists | dmc parser handles natively | `remark-gfm` |
| heading slug + anchor | `<h2 id="..."><a href="#..." class="subheading-anchor">` (dmc default) | `<h2 id="..."><a href="#...">` (rehype-autolink-headings) |
| bare URL autolink | `BareUrlAutolink` transformer | `remark-gfm` |

Reload, view source, diff the served HTML.

## Known divergences

- **Math glyphs**: dmc emits MathML; browsers render it directly. velite
  emits KaTeX HTML and pulls `katex.min.css` from a CDN (configured in
  `app/layout.tsx`). Visual output is similar, but the DOM structure
  differs.
- **Theme set**: dmc bundles `Catppuccin Latte`/`Catppuccin Mocha`
  (light + dark). velite uses shiki's `catppuccin-latte` /
  `catppuccin-mocha` themes. Colors match in light mode.
- **HTML size**: dmc kitchen-sink HTML is about 12 KB; velite HTML is
  about 25 KB (KaTeX expansion + pretty-code's `<figure>` wrappers).
- **Build time**: see `docs/sidecar-path-perf.md` for benchmark numbers.

## Files of interest

- `examples/nextjs/duck-md.config.ts` - dmc config; note `output.html: true`
- `examples/nextjs/scripts/build-content.ts` - calls `build()` from `@gentleduck/md`
- `examples/nextjs-velite/velite.config.ts` - velite config; the JS plugin chain
- `examples/nextjs/app/docs/[slug]/page.tsx` and the velite mirror -
  identical shape, different dataset imports.
