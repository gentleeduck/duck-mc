import { defineConfig } from "@gentleduck/md";

export default defineConfig({
  outputDir: ".gentleduck",
  collections: [
    {
      name: "docs",
      pattern: "docs/**/*.mdx",
      baseDir: "content",
    },
  ],
});
