import duck from './mod.js'
import { mkdtempSync, mkdirSync, writeFileSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import { tmpdir } from 'node:os'

const { build, defineConfig, s } = duck

const dir = mkdtempSync(join(tmpdir(), 'duck-cbs-'))
mkdirSync(join(dir, 'docs'), { recursive: true })
writeFileSync(join(dir, 'docs', 'a.mdx'), `---\ntitle: hello\nhandle: my-post\n---\n# A\n`)
writeFileSync(join(dir, 'docs', 'b.mdx'), `---\ntitle: world\nhandle: bad handle here\n---\n# B\n`)

const cfg = defineConfig({
  root: dir,
  output: { data: join(dir, '.gentleduck'), clean: true },
  collections: {
    docs: {
      name: 'doc',
      pattern: 'docs/**/*.mdx',
      schema: s.object({
        title: s.string().transform((v) => (v as string).toUpperCase()),
        handle: s.string().refine((v) => !(v as string).includes(' '), 'handle must not contain spaces'),
      }),
    },
  },
})

const rep = await build(cfg)
console.log('errors:', rep.errors)
const records = JSON.parse(readFileSync(rep.collections[0].outputPath, 'utf8'))
console.log('record titles:', (records as { title: string }[]).map((r) => r.title))
