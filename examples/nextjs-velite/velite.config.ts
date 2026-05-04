import { defineConfig, s } from "velite";
import remarkGfm from "remark-gfm";
import remarkMath from "remark-math";
import remarkEmoji from "remark-emoji";
import rehypeKatex from "rehype-katex";
import rehypeSlug from "rehype-slug";
import rehypeAutolinkHeadings from "rehype-autolink-headings";
import rehypePrettyCode from "rehype-pretty-code";

export default defineConfig({
  root: "content",
  output: {
    data: ".velite",
    assets: "public/assets",
    base: "/assets/",
    name: "[name]-[hash:6].[ext]",
    clean: true,
  },
  collections: {
    docs: {
      name: "Doc",
      pattern: "docs/**/*.mdx",
      schema: s
        .object({
          title: s.string().max(99),
          description: s.string().optional(),
          tags: s.array(s.string()).optional(),
          slug: s.path(),
          html: s.markdown(),
        })
        .transform((data) => ({ ...data, permalink: data.slug.replace(/^docs\//, "") })),
    },
  },
  // unified's plugin generic types diverge slightly across major
  // versions, so we widen here to keep the example install lean
  // (no per-plugin override types).
  // biome-ignore lint/suspicious/noExplicitAny: see comment above
  markdown: {
    remarkPlugins: [
      remarkGfm,
      remarkMath,
      [remarkEmoji, { emoticon: false, accessible: false }],
    ],
    rehypePlugins: [
      [
        rehypePrettyCode,
        {
          theme: { light: "catppuccin-latte", dark: "catppuccin-mocha" },
          keepBackground: true,
        },
      ],
      rehypeSlug,
      [rehypeAutolinkHeadings, { behavior: "wrap" }],
      rehypeKatex,
    ],
    // biome-ignore lint/suspicious/noExplicitAny: cast widens to bypass unified
    // ^^^ vendored types skew between remark-* majors.
  } as any,
});
