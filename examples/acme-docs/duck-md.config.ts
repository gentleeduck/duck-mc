import { defineConfig, definePlugin, s } from '@duck/md'
import remarkGfm from 'remark-gfm'
import rehypeSlug from 'rehype-slug'
import rehypeAutolinkHeadings from 'rehype-autolink-headings'
import rehypePrettyCode from 'rehype-pretty-code'

export default defineConfig({
  root: '.',
  output: { data: '.gentleduck', clean: true, html: true } as never,
  mdx: {
    remarkPlugins: [remarkGfm],
    rehypePlugins: [
      rehypeSlug,
      definePlugin(rehypePrettyCode, {
        theme: { light: 'github-light', dark: 'catppuccin-mocha' },
        keepBackground: false,
      }),
      definePlugin(rehypeAutolinkHeadings, {
        properties: { className: ['subheading-anchor'], 'aria-label': 'Link to section' },
      }),
    ],
  },
  collections: {
    docs: {
      name: 'doc',
      pattern: '**/*.mdx',
      baseDir: 'content/docs',
      schema: s.object({
        title: s.string().max(99),
        description: s.string().optional(),
      }),
    },
  },
})
