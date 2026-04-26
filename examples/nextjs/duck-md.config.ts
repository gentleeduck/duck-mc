import { defineConfig, s } from "@duck/md";

export default defineConfig({
  root: "content",
  output: { data: ".gentleduck" },
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
