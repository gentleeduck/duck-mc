import { defineConfig, s } from '@duck/md'

export default defineConfig({
  root: '.',
  output: { data: '.gentleduck', clean: true, html: true } as never,
  mdx: {
    remarkPlugins: [
      ['remark-gfm'],
    ],
    rehypePlugins: [
      ['rehype-slug'],
      ['rehype-pretty-code', {
        theme: { light: 'github-light', dark: 'catppuccin-mocha' },
        keepBackground: false,
      }],
      ['rehype-autolink-headings', {
        properties: { className: ['subheading-anchor'], 'aria-label': 'Link to section' },
      }],
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
