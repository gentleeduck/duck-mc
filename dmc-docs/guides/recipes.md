# Recipes

Copy-paste solutions for common needs.

## Sort posts by date desc

```ts
defineConfig({
  collections: {
    posts: {
      name: "post",
      pattern: "posts/**/*.mdx",
      schema: s.object({ title: s.string(), date: s.date() }),
    },
  },
  prepare(data) {
    const posts = data.posts as Array<{ date: string }>;
    posts.sort((a, b) => +new Date(b.date) - +new Date(a.date));
  },
});
```

## Drop drafts in production

```ts
prepare(data) {
  if (process.env.NODE_ENV === "production") {
    data.posts = (data.posts as Array<{ draft?: boolean }>).filter(
      p => !p.draft,
    );
  }
},
```

## Auto-generate slug

```ts
collections: {
  posts: {
    name: "post",
    pattern: "posts/**/*.mdx",
    schema: s
      .object({
        title: s.string(),
        slug: s.path().optional(),
      })
      .transform((data, ctx) => ({
        ...data,
        slug: data.slug ?? slugify(data.title),
      })),
  },
},
```

## Generate RSS in `complete` hook

```ts
import { writeFileSync } from "fs";
import RSS from "rss";

complete(data) {
  const feed = new RSS({ title: "Site", site_url: "https://x.com" });
  for (const p of data.posts as Array<{ title: string; date: string; html: string; permalink: string }>) {
    feed.item({
      title: p.title,
      url: `https://x.com/${p.permalink}`,
      description: p.html,
      date: p.date,
    });
  }
  writeFileSync("public/rss.xml", feed.xml({ indent: true }));
},
```

## Reading time + word count

Already produced by `Accumulator`. Read off the record:

```ts
const post = posts[0];
console.log(`${post.metadata.wordCount} words, ${post.metadata.readingTime} min`);
```

## Excerpt

```ts
const post = posts[0];
console.log(post.excerpt);   // first ~200 chars of plain text
```

Customise the cap by post-processing in `prepare`:

```ts
prepare(data) {
  for (const p of data.posts) {
    p.excerpt = (p.content as string).split(/\s+/).slice(0, 50).join(" ");
  }
},
```

## Custom heading slug

`AutolinkHeadings` slugifies via `slug::slugify`. To override per
record:

```ts
prepare(data) {
  for (const post of data.posts as Array<{ html: string; slug?: string }>) {
    if (post.slug) {
      post.html = post.html.replace(/id="[^"]+"/, `id="${post.slug}"`);
    }
  }
},
```

Ugly. For a real solution, write a custom transformer that overrides
heading slugs based on a frontmatter `slug` field.

## TOC for sidebar

Already produced by `Accumulator`. Use directly:

```tsx
import { post } from "../.gentleduck";

function Sidebar() {
  return (
    <ul>
      {post[0].toc.map(t => (
        <li key={t.url}>
          <a href={t.url}>{t.title}</a>
          {t.items.length > 0 && <Sidebar items={t.items} />}
        </li>
      ))}
    </ul>
  );
}
```

## Multi-theme code blocks

```ts
defineConfig({
  prettyCode: {
    theme: { light: "Catppuccin Latte", dark: "Catppuccin Mocha" },
    defaultMode: "dark",
  },
});
```

CSS:

```css
html.light pre,
html.light pre code,
html.light pre code span {
  color: var(--dmc-light);
  background-color: var(--dmc-light-bg);
}
```

Hook the toggle to `<html class="light"|"dark">`.

## Ship MathML instead of KaTeX (smaller HTML, faster build)

```ts
mathEngine: "mathml"
```

Drop the KaTeX CSS link. MathML renders natively.

## Image processing on copy

Built-in `copy-linked-files` only copies + hashes. For resize /
format conversion, write a custom transformer that walks `Node::Image`
and shells out to `sharp` (via Node) or runs an in-process Rust
image crate.

## Foreign rehype plugin

```ts
import rehypeExternalLinks from "rehype-external-links";

defineConfig({
  markdown: {
    rehypePlugins: [
      [rehypeExternalLinks, { target: "_blank", rel: ["nofollow", "noopener"] }],
    ],
  },
});
```

Runs in dmc-sidecar Node child. Adds ~one round-trip per file unless
the gate strips every plugin.

## Custom file type via loader

```ts
import { defineLoader } from "@duck/md";
import yaml from "js-yaml";

const yamlLoader = defineLoader({
  test: /\.ya?ml$/,
  load({ value }) {
    return { data: yaml.load(value) };
  },
});

defineConfig({
  loaders: [yamlLoader],
  collections: {
    settings: {
      name: "setting",
      pattern: "settings/*.yaml",
      schema: s.object({ name: s.string() }),
    },
  },
});
```

## Render once, ship to multiple frameworks

```ts
defineConfig({
  output: { html: true },
  collections: {
    docs: {
      name: "doc",
      pattern: "docs/**/*.mdx",
      schema: s.object({ title: s.string(), content: s.markdown() }),
    },
  },
});
```

`s.markdown()` renders the body during validation. Use `doc[i].content`
as raw HTML in any framework (Next.js, Astro, SvelteKit, plain HTML).

## Test a transformer

```rust
use dmc_parser::parse;
use dmc_transform::Pipeline;

#[test]
fn drops_javascript_images() {
    let mut doc = parse("![bad](javascript:alert(1))");
    Pipeline::new().add(MyPass).run_silent(&mut doc);
    let count = doc.children.iter().filter(|n| matches!(n, dmc_parser::ast::Node::Image(_))).count();
    assert_eq!(count, 0);
}
```

`run_silent` synthesises meta + throwaway diag engine.

## Programmatic compile (one-shot)

```ts
import { compile } from "@duck/md";

const out = compile(`---
title: hi
---
# heading`);

console.log(out.html);
console.log(out.frontmatter);
```

No file cache, no sidecar; pure compile.
