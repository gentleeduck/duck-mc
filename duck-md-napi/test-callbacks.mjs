import duck from './mod.mjs'
import fs from 'node:fs'
import path from 'node:path'
import os from 'node:os'

const { build, defineConfig, s } = duck

const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'duck-cbs-'))
fs.mkdirSync(path.join(dir, 'docs'), { recursive: true })
fs.writeFileSync(path.join(dir, 'docs', 'a.mdx'), `---\ntitle: hello\nhandle: my-post\n---\n# A\n`)
fs.writeFileSync(path.join(dir, 'docs', 'b.mdx'), `---\ntitle: world\nhandle: bad handle here\n---\n# B\n`)

const cfg = defineConfig({
  root: dir,
  output: { data: path.join(dir, '.gentleduck'), clean: true },
  collections: {
    docs: {
      name: 'doc',
      pattern: 'docs/**/*.mdx',
      schema: s.object({
        title: s.string().transform(v => v.toUpperCase()),
        handle: s.string().refine(v => !v.includes(' '), 'handle must not contain spaces'),
      }),
    },
  },
})

const rep = await build(cfg)
console.log('errors:', rep.errors)
const records = JSON.parse(fs.readFileSync(rep.collections[0].outputPath, 'utf8'))
console.log('record titles:', records.map(r => r.title))
console.log('record slugs:', records.map(r => r.slug))
