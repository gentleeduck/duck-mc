# Remix integration

Remix runs Vite under the hood (Remix 2.x). Wire dmc into the Vite
build the same way as raw Vite, then read the generated content
modules from your routes.

## Install

```bash
pnpm add -D @duck/md
```

## Vite plugin

`vite.config.ts`:

```ts
import { vitePlugin as remix } from "@remix-run/dev";
import { defineConfig } from "vite";
import { dmc } from "@duck/md/vite";

export default defineConfig({
  plugins: [
    dmc({
      configFile: "./dmc.config.ts",
      watch: true,
    }),
    remix(),
  ],
});
```

`dmc()` runs `Engine::run` once on `buildStart` and again on
`watchChange` for any file under any collection's pattern.

## dmc config

`dmc.config.ts`:

```ts
import { defineConfig } from "@duck/md";

export default defineConfig({
  output: { data: ".dmc", clean: true },
  collections: {
    posts: {
      name: "Post",
      pattern: "content/posts/**/*.mdx",
      schema: (s) =>
        s.object({
          title: s.string(),
          date: s.isodate(),
          slug: s.path(),
        }),
    },
  },
});
```

## Read content in a loader

`app/routes/posts.$slug.tsx`:

```tsx
import { json, type LoaderFunctionArgs } from "@remix-run/node";
import { useLoaderData } from "@remix-run/react";
import posts from "../../.dmc/Post.json";

export async function loader({ params }: LoaderFunctionArgs) {
  const post = posts.find((p) => p.slug === params.slug);
  if (!post) throw new Response("Not Found", { status: 404 });
  return json({ post });
}

export default function PostRoute() {
  const { post } = useLoaderData<typeof loader>();
  return (
    <article dangerouslySetInnerHTML={{ __html: post.content }} />
  );
}
```

## MDX components

If you used MDX components (the `mdx` output format), get the
component factory:

```tsx
import * as runtime from "react/jsx-runtime";
import { runSync } from "@mdx-js/mdx";

const { default: MDXContent } = runSync(post.body, runtime);
```

The `body` field on the record is the JSX-compiled module source.
`@mdx-js/mdx`'s `runSync` builds a component you can render.

For static HTML output, skip this; use the `content` field directly.

## Caveats

- Remix server runtime is Node by default. dmc's `.dmc/*.json`
  imports work as plain JSON imports. No platform-specific changes.
- Cloudflare Workers / Deno deploy: use the JSON output (no
  `runSync` of MDX at runtime; pre-compile the body server-side at
  build time).
- Hot reload in dev: Vite's HMR picks up `.dmc/*.json` changes
  because dmc rewrites the file. Loader re-imports on next request.

## Where to put `.dmc`

Default `output.data = ".dmc"` lives at the project root. Add to
`.gitignore`:

```gitignore
.dmc/
```

Remix bundles only `app/` by default; the `.dmc/` directory is
imported via relative paths so it ends up in the bundle the same
way `node_modules` does.

## TS types

dmc emits `.dmc/index.d.ts` alongside the JSON. Add to
`tsconfig.json` paths:

```json
{
  "compilerOptions": {
    "paths": {
      "~/dmc": ["./.dmc/index"]
    }
  }
}
```

Then:

```ts
import { posts } from "~/dmc";
```

Records are typed via the schema you defined.
