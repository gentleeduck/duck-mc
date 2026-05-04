import { defineConfig, s } from "@gentleduck/md";

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
        tags: s.array(s.string()).optional(),
      }),
    },
  },
});
