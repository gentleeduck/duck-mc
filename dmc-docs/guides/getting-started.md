# Getting started

## Install

```bash
pnpm add @duck/md
```

Pulls the prebuilt `*.node` binary for your platform.

## Scaffold

```bash
pnpm dmc init
```

Writes:
- `duck-md.config.ts`
- `content/docs/index.mdx`
- `.gitignore` line for `.gentleduck/`

Skip if you already have a config.

## Minimal config

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
        title: s.string().max(99),
        description: s.string().optional(),
      }),
    },
  },
});
```

| field | use |
|-------|-----|
| `root` | content folder |
| `output.data` | dir for `<name>.json` + `index.{js,d.ts}` |
| `output.html` | include rendered HTML on each record |
| `collections.<key>` | one entry per output file |
| `schema` | Zod-style frontmatter validation |

## Build

```bash
pnpm dmc build
```

Writes `.gentleduck/doc.json` + `.gentleduck/index.js` +
`.gentleduck/index.d.ts`.

## Watch

```bash
pnpm dmc dev
```

Rebuilds on file change. Persistent cache means only modified files
re-compile.

## Use the output

```ts
import { doc } from "../.gentleduck";

console.log(doc[0].title);
console.log(doc[0].html);
```

ESM by default. Change via `output.format: "cjs"`.

## Next.js example

See `examples/nextjs/` in the repo. Key pieces:

```ts
// app/docs/[slug]/page.tsx
import docs from "../../../.gentleduck/doc.json";

export function generateStaticParams() {
  return docs.map((d) => ({ slug: d.permalink.replace(/^docs\//, "") }));
}

export default async function Page({ params }) {
  const { slug } = await params;
  const doc = docs.find((d) => d.permalink === `docs/${slug}`);
  return <article dangerouslySetInnerHTML={{ __html: doc.html }} />;
}
```

`predev` and `prebuild` scripts run `pnpm dmc build` so Next.js sees a
populated `.gentleduck` dir.
