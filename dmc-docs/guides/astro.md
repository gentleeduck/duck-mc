# Astro

Use dmc as a content provider for an Astro site. Same JSON-output
shape Astro's content collections expect.

## Install

```bash
pnpm add @duck/md
```

## Config

```ts
// duck-md.config.ts
import { defineConfig, s } from "@duck/md";

export default defineConfig({
  root: "src/content",
  output: { data: ".gentleduck", html: true },
  collections: {
    blog: {
      name: "post",
      pattern: "blog/**/*.mdx",
      schema: s.object({
        title: s.string(),
        pubDate: s.date(),
        description: s.string().optional(),
      }),
    },
  },
});
```

## Build script

```ts
// scripts/build-content.ts
import { build } from "@duck/md";
import config from "../duck-md.config";
await build(config);
```

## package.json

```json
{
  "scripts": {
    "content": "tsx scripts/build-content.ts",
    "predev": "pnpm content",
    "prebuild": "pnpm content",
    "dev": "astro dev",
    "build": "astro build"
  }
}
```

## Page

```astro
---
// src/pages/blog/[...slug].astro
import { post } from "../../../.gentleduck";

export function getStaticPaths() {
  return post.map((p) => ({
    params: { slug: p.permalink.replace(/^blog\//, "") },
    props: { p },
  }));
}

const { p } = Astro.props;
---

<html>
  <body>
    <h1>{p.title}</h1>
    <article set:html={p.html} />
  </body>
</html>
```

## With KaTeX

Add the CSS to your layout:

```astro
---
// src/layouts/Base.astro
---
<html>
  <head>
    <link
      rel="stylesheet"
      href="https://cdn.jsdelivr.net/npm/katex@0.16.11/dist/katex.min.css"
    />
  </head>
  <body><slot /></body>
</html>
```

## Watch mode

Run `dmc dev` in a second terminal so `.gentleduck/*.json` updates
trigger Astro HMR.

## Trade-offs vs Astro content collections

| | Astro CC | dmc |
|-|---------|-----|
| MDX support | yes (via `@astrojs/mdx`) | yes (native) |
| schema | Zod | Zod-style |
| typed collection | yes | yes |
| persistent cache | no | yes |
| code highlight | shiki (built-in) | syntect (faster) |
| math | external plugins | native |
| build speed | depends on plugins | 9.5x velite kitchen-sink |

dmc trades a small extra build step for big wins on perf + caching.
Astro CC's tight integration is simpler if perf is not a concern.
