import { defineConfig, s } from '@duck/md'

export default defineConfig({
  root: '.',
  output: { data: '.gentleduck', clean: true, html: true } as never,
  mdx: {
    rehypePlugins: [
      ['rehype-slug'],
      ['rehype-autolink-headings', {
        behavior: 'wrap',
        properties: { className: ['subheading-anchor'], 'aria-label': 'Link to section' },
      }],
      ['rehype-pretty-code', {
        theme: { light: 'github-light', dark: 'catppuccin-mocha' },
        keepBackground: false,
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
