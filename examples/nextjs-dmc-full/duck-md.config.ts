import { defineConfig, s } from "@gentleduck/md";

// Mirror duck-ui's docs setup: per-package collections, velite-compat
// schema with `s.transform`, MDX body emission for runtime React rendering
// (instead of pre-rendered HTML).
const docFields = {
  title: s.string().max(99),
  description: s.string().optional(),
  tags: s.array(s.string()).optional(),
};

const transformSlug = (data: any, ctx: { meta: { path: string } }) => ({
  ...data,
  slug: ctx.meta.path.replace(/\.mdx$/, ""),
  permalink: ctx.meta.path
    .replace(/^content\//, "")
    .replace(/\.mdx$/, ""),
});

export default defineConfig({
  root: "content",
  // Emit MDX bodies (jsx-runtime callable) plus pre-rendered HTML so we
  // can compare both pipelines on the same docset.
  output: { data: ".gentleduck", html: true, clean: true },
  collections: {
    duckUi: {
      name: "DuckUi",
      pattern: "docs/duck-ui/**/*.mdx",
      schema: s.object(docFields).transform(transformSlug),
    },
    duckHooks: {
      name: "DuckHooks",
      pattern: "docs/duck-hooks/**/*.mdx",
      schema: s.object(docFields).transform(transformSlug),
    },
  },
});
