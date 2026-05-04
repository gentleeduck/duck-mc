# SvelteKit

Use dmc as the content layer for a SvelteKit app. JSON output works
the same as in Next.js / Astro.

## Install

```bash
pnpm add @gentleduck/md
```

## Config

```ts
// duck-md.config.ts
import { defineConfig, s } from "@gentleduck/md";

export default defineConfig({
  root: "src/content",
  output: { data: ".gentleduck", html: true },
  collections: {
    posts: {
      name: "post",
      pattern: "posts/**/*.mdx",
      schema: s.object({
        title: s.string(),
        date: s.date(),
        description: s.string().optional(),
      }),
    },
  },
});
```

## Build script + scripts

```ts
// scripts/build-content.ts
import { build } from "@gentleduck/md";
import config from "../duck-md.config";
await build(config);
```

```json
{
  "scripts": {
    "content": "tsx scripts/build-content.ts",
    "predev": "pnpm content",
    "prebuild": "pnpm content",
    "dev": "vite dev",
    "build": "vite build",
    "preview": "vite preview"
  }
}
```

## Listing page

```svelte
<!-- src/routes/blog/+page.svelte -->
<script lang="ts">
  import { post } from "../../../.gentleduck";
</script>

<h1>Blog</h1>
<ul>
  {#each post as p}
    <li>
      <a href="/blog/{p.permalink}">{p.title}</a>
      <small>{p.date}</small>
    </li>
  {/each}
</ul>
```

## Per-post page

```svelte
<!-- src/routes/blog/[slug]/+page.svelte -->
<script lang="ts">
  import type { PageData } from "./$types";
  export let data: PageData;
</script>

<article>
  <h1>{data.post.title}</h1>
  {@html data.post.html}
</article>
```

```ts
// src/routes/blog/[slug]/+page.ts
import { error } from "@sveltejs/kit";
import { post } from "../../../../.gentleduck";

export function load({ params }) {
  const found = post.find((p) => p.permalink === `posts/${params.slug}`);
  if (!found) throw error(404, "Not found");
  return { post: found };
}

export const prerender = true;
```

## Static prerendering

```ts
// src/routes/blog/[slug]/+page.ts
import { post } from "../../../../.gentleduck";

export const prerender = true;
export const entries = () =>
  post.map((p) => ({ slug: p.permalink.replace(/^posts\//, "") }));
```

SvelteKit reads `entries` at build time and prerenders one HTML file
per slug. dmc's persistent cache means warm rebuilds skip compile.

## KaTeX styles

Add to root layout:

```svelte
<!-- src/routes/+layout.svelte -->
<svelte:head>
  <link
    rel="stylesheet"
    href="https://cdn.jsdelivr.net/npm/katex@0.16.11/dist/katex.min.css"
  />
</svelte:head>

<slot />
```

## Watch mode

```bash
# terminal 1
pnpm dmc dev

# terminal 2
pnpm dev
```

Vite HMR picks up JSON changes; SvelteKit reloads affected routes.

## Tradeoffs

| | mdsvex | dmc |
|-|--------|-----|
| MDX-in-Svelte | yes (via mdsvex transform) | no (HTML only via {@html}) |
| build speed | per-file svelte compile | one JSON read at runtime |
| code highlight | shiki via mdsvex highlighter | syntect (faster) |
| schema validation | none | Zod-style |

mdsvex gives you Svelte-in-MDX. dmc gives you fast SSG with cached
JSON. Pick by what you actually need.
