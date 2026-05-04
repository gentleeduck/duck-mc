# Vite

dmc plugs into any Vite-based app (React, Solid, Vue, vanilla) the
same way: build content to JSON before vite starts, import the JSON.

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
    docs: {
      name: "doc",
      pattern: "**/*.mdx",
      schema: s.object({
        title: s.string(),
        order: s.number().optional(),
      }),
    },
  },
});
```

## package.json

```json
{
  "scripts": {
    "content": "tsx scripts/build-content.ts",
    "predev": "pnpm content",
    "prebuild": "pnpm content",
    "dev": "vite",
    "build": "vite build",
    "preview": "vite preview"
  }
}
```

## Use the JSON

```ts
// src/main.ts
import { doc } from "../.gentleduck";

document.querySelector<HTMLDivElement>("#app")!.innerHTML = `
  <ul>
    ${doc.map(d => `<li><a href="/${d.permalink}">${d.title}</a></li>`).join("")}
  </ul>
`;
```

## React

```tsx
import { doc } from "../.gentleduck";

export function DocList() {
  return (
    <ul>
      {doc.map((d) => (
        <li key={d.permalink}>
          <a href={`/${d.permalink}`}>{d.title}</a>
        </li>
      ))}
    </ul>
  );
}
```

## Vite plugin route (optional)

For HMR on content edits, add a small Vite plugin that watches
`.gentleduck/`:

```ts
// vite.config.ts
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [
    {
      name: "dmc-content-hmr",
      configureServer(server) {
        server.watcher.add(".gentleduck/**/*.json");
      },
    },
  ],
});
```

Pair with `dmc dev` in a second terminal so JSON updates trigger
HMR reload.

## SSG

Generate static HTML from the JSON in your build step. Vite is
client-side by default; for true SSG use Vite SSR or a meta-framework
like Astro / SvelteKit / Remix on top.

## Watch mode

```bash
# terminal 1
pnpm dmc dev

# terminal 2
pnpm dev
```

The dmc dev server rebuilds JSON on MDX changes; vite picks up the
JSON change via HMR.
