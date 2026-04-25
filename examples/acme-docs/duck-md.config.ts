import { defineConfig, s } from '@duck/md'

export default defineConfig({
  root: '.',
  output: { data: '.gentleduck', clean: true, html: true } as never,
  mdx: { themeLight: 'github-light', themeDark: 'catppuccin-mocha' } as never,
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
