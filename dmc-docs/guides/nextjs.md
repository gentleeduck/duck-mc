# Next.js

End-to-end recipe. See `examples/nextjs/` for a working app.

## Install

```bash
cd my-next-app
pnpm add @duck/md
```

## Config

```ts
// duck-md.config.ts
import { defineConfig, s } from "@duck/md";

export default defineConfig({
  root: "content",
  output: { data: ".gentleduck", html: true },
  collections: {
    docs: {
      name: "doc",
      pattern: "docs/**/*.mdx",
      schema: s.object({
        title: s.string(),
        description: s.string().optional(),
        tags: s.array(s.string()).optional(),
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

const report = await build(config);
const collections = report?.collections ?? [];
for (const c of collections) {
  console.log(`+ ${c.name}: ${c.records} -> ${c.outputPath}`);
}
```

## package.json

```json
{
  "scripts": {
    "content": "tsx scripts/build-content.ts",
    "predev": "pnpm content",
    "prebuild": "pnpm content",
    "dev": "next dev",
    "build": "next build",
    "start": "next start"
  }
}
```

`predev` and `prebuild` run dmc before Next.js starts, so the
`.gentleduck/` JSON is populated when Next imports it.

## Static-render slug page

```tsx
// app/docs/[slug]/page.tsx
import docs from "../../../.gentleduck/doc.json";
import { notFound } from "next/navigation";

type Doc = (typeof docs)[number] & { html?: string };

const stripDir = (p: string) => p.replace(/^docs\//, "");

export function generateStaticParams() {
  return docs.map((d) => ({ slug: stripDir(d.permalink) }));
}

export default async function Page({
  params,
}: {
  params: Promise<{ slug: string }>;
}) {
  const { slug } = await params;
  const doc = docs.find((d) => stripDir(d.permalink) === slug) as Doc | undefined;
  if (!doc) notFound();
  return (
    <article>
      <h1>{doc.title}</h1>
      <div dangerouslySetInnerHTML={{ __html: doc.html ?? "" }} />
    </article>
  );
}
```

`output.html: true` ensures every record has the rendered HTML field.
`generateStaticParams` makes Next pre-render every doc at build time.

## Layout

```tsx
// app/layout.tsx
import "./globals.css";
import type { ReactNode } from "react";

export default function RootLayout({ children }: { children: ReactNode }) {
  return (
    <html lang="en">
      <head>
        <link
          rel="stylesheet"
          href="https://cdn.jsdelivr.net/npm/katex@0.16.11/dist/katex.min.css"
        />
      </head>
      <body>{children}</body>
    </html>
  );
}
```

KaTeX CSS is needed when `mathEngine: "katex"` (default). Drop the
link tag when using `mathEngine: "mathml"`.

## Watch mode

```bash
pnpm dev
```

Next.js watches its own files; dmc's predev runs once. For live MDX
editing, run `pnpm dmc dev` in a second terminal so the
`.gentleduck/*.json` updates trigger Next.js HMR.

## Type flow

`@duck/md` writes `.gentleduck/index.d.ts` that re-exports each
collection's record type via `typeof import(config)`. So:

```ts
import { doc } from "../.gentleduck";
//      ^? Doc[]
```

is fully typed without manual annotations.

## Prod considerations

- `output.html: true` on the records means HTML is in your JSON,
  shipped to the client. Around 12 KB per kitchen-sink page. Skip
  via `false` if you render MDX runtime-side (use `body` field +
  `@mdx-js/react` runtime).
- Cache: `.gentleduck/.cache/` makes warm rebuilds 3.55x faster.
  Persist between CI runs (e.g. via Vercel's build cache).
- Serverless deploys ship the static `.gentleduck/*.json`; no Rust
  binary at runtime.

## Working example

See `examples/nextjs/` and `examples/nextjs-velite/` for side-by-side
demos. `COMPARISON.md` explains how to run both at once.
