# dmc-napi examples

## Minimal config

```ts
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
      }),
    },
  },
});
```

## Multi-collection with hooks

```ts
import { defineConfig, s } from "@duck/md";

export default defineConfig({
  root: "content",
  output: { data: ".gentleduck", html: true },
  cacheEnabled: true,
  collections: {
    docs: {
      name: "doc",
      pattern: "docs/**/*.mdx",
      schema: s.object({
        title: s.string().max(99),
        description: s.string().optional(),
        order: s.number().optional(),
      }),
    },
    posts: {
      name: "post",
      pattern: "posts/**/*.mdx",
      schema: s.object({
        title: s.string(),
        date: s.date(),
        draft: s.boolean().optional(),
      }),
    },
  },
  prepare(data) {
    const posts = data.posts as Array<{ draft?: boolean; date: string }>;
    posts.sort((a, b) => +new Date(b.date) - +new Date(a.date));
    data.posts = posts.filter(p => !p.draft);
  },
});
```

## Programmatic build (Node)

```ts
import { build } from "@duck/md";
import config from "./duck-md.config";

const report = await build(config);
for (const c of report.collections) {
  console.log(`${c.name}: ${c.records} records -> ${c.outputPath}`);
}
if (report.errors.length) {
  process.exitCode = 1;
}
```

## One-shot compile

```ts
import { compile } from "@duck/md";

const out = compile(`---
title: Hello
---

# hello *world*

\`\`\`rust
fn main() {}
\`\`\`
`);

console.log(out.html);
```

Synchronous. Skips file cache + sidecar. Useful for unit tests, REPL,
or programmatic doc inspection.

## Direct LaTeX render

```ts
import { latexToHtml } from "@duck/md";

const inline = latexToHtml("E = mc^2", false);
const block  = latexToHtml("\\int_0^1 x^2 \\, dx", true);
```

Useful in custom remark plugins or build steps that want the same KaTeX
output as the dmc compile path.
