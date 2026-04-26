import { definePlugin } from "@duck/md";
import rehypePrettyCode from "rehype-pretty-code";
import rehypeAutolinkHeadings from "rehype-autolink-headings";

// SHOULD type-check (correct options shape)
const ok = definePlugin(rehypePrettyCode, {
  theme: { light: "github-light", dark: "catppuccin-mocha" },
});

// SHOULD type-error: wrong field for rehypePrettyCode
// @ts-expect-error
const wrong = definePlugin(rehypePrettyCode, { totallyWrongField: 123 });

// SHOULD type-error: wrong field for rehypeAutolinkHeadings
// @ts-expect-error
const wrong2 = definePlugin(rehypeAutolinkHeadings, { nonexistent: "x" });

console.log({ ok, wrong, wrong2 });
