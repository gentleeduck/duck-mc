# JS-side patterns

## Hooks

```ts
defineConfig({
  collections: { /* ... */ },
  prepare(data) {
    // mutate or filter records before write
    data.posts = (data.posts as Post[]).filter(p => !p.draft);
  },
  complete(data) {
    // last-stage hook; final shape ready
    fs.writeFileSync("posts.rss", buildRss(data.posts));
  },
});
```

`prepare` runs after schema validation, before JSON write.
`complete` runs after every collection has been written.

## Collection callbacks

```ts
import { defineCollection } from "@gentleduck/md";

const posts = defineCollection({
  name: "post",
  pattern: "posts/**/*.mdx",
  schema: s.object({ title: s.string(), date: s.date() }),
  onRecord(rec) {
    rec.permalink = `/blog/${rec.slug}`;
    return rec;
  },
});
```

`onRecord` runs once per file after schema validation. Return value
replaces the record.

## Custom loaders

```ts
import { defineLoader } from "@gentleduck/md";

const yaml = defineLoader({
  test: /\.ya?ml$/,
  load({ path, value }) {
    return { data: parseYaml(value) };
  },
});

defineConfig({
  loaders: [yaml],
  // ...
});
```

Loader picks up files whose `path` matches its `test` regex; returns
parsed `data` for the engine to feed through schema validation.

## Plugins (foreign)

```ts
import remarkFrontmatter from "remark-frontmatter";
import rehypeSlug from "rehype-slug";

defineConfig({
  markdown: {
    remarkPlugins: [remarkFrontmatter],
    rehypePlugins: [rehypeSlug],   // stripped at gate; native handles it
  },
  // ...
});
```

Listed plugins run in the dmc-sidecar Node child. Plugins owned by
native transformers are stripped before dispatch (`remark-gfm`,
`remark-math`, `rehype-katex`, `rehype-pretty-code`, `shiki`,
`rehype-slug`, `rehype-autolink-headings`, `remark-emoji`).

If after stripping nothing remains, the sidecar is never spawned.

### Force a JS plugin (override the gate)

Two knobs on `markdown` / `mdx` blocks:

```ts
import rehypeKatex from "rehype-katex";

defineConfig({
  markdown: {
    rehypePlugins: [
      [rehypeKatex, { strict: false, trust: true }],
    ],
    preferSidecar: ["rehype-katex"],   // run katex in sidecar, drop native Math
  },
});
```

`preferSidecar` keeps the listed names in the sidecar payload AND
drops the matching native transformer from the pipeline. No double
work. Recognised names: `remark-gfm`, `remark-math`, `remark-emoji`,
`rehype-pretty-code`, `shiki`, `rehype-katex`, `rehype-mathjax`,
`rehype-slug`, `rehype-autolink-headings`.

For the global hammer:

```ts
defineConfig({
  markdown: {
    rehypePlugins: [/* whatever */],
    forceSidecar: true,   // every JS plugin in sidecar, all natives dropped
  },
});
```

Equivalent to listing every recognised name in `preferSidecar`.

## Static asset copy

```ts
defineConfig({
  output: { assets: "public/assets", base: "/assets/" },
  markdown: { copyLinkedFiles: true },
  // ...
});
```

`![](./img.png)` and `[file](./doc.pdf)` get copied to the assets dir
with hash-named filenames; the rendered `src`/`href` is rewritten to
the `base` prefix.

## Single-record collection

```ts
defineConfig({
  collections: {
    site: defineCollection({
      name: "site",
      pattern: "site.mdx",
      single: true,
      schema: s.object({ title: s.string(), nav: s.array(s.string()) }),
    }),
  },
});
```

`single: true` emits one object instead of an array.
