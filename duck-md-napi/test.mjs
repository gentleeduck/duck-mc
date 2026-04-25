import duck from './mod.mjs'
import fs from 'node:fs'
import path from 'node:path'
import os from 'node:os'

const { compile, build, defineConfig, s } = duck

const out = compile(`---\ntitle: Hi\n---\n\n# Hello **world**\n\n- a\n- b\n`)
console.log('compile.title:', out.frontmatter?.title)
console.log('compile.metadata:', out.metadata)
console.log('compile.toc:', JSON.stringify(out.toc))

const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'duck-md-napi-'))
fs.mkdirSync(path.join(dir, 'docs'), { recursive: true })
fs.writeFileSync(path.join(dir, 'docs', 'a.mdx'), `---\ntitle: A\n---\n# Alpha\n`)
fs.writeFileSync(path.join(dir, 'docs', 'b.mdx'), `---\ntitle: B\n---\n# Beta\n`)

// velite-shape config (records map, output.data, schema descriptors)
const cfg = defineConfig({
  root: dir,
  output: { data: path.join(dir, '.gentleduck'), clean: true },
  collections: {
    docs: {
      name: 'Doc',
      pattern: 'docs/**/*.mdx',
      schema: s.object({
        title: s.string().max(99),
        draft: s.boolean().default(false),
      }),
    },
  },
})

const rep = build(cfg)
console.log('build.report:', JSON.stringify(rep, null, 2))

const json = JSON.parse(fs.readFileSync(rep.collections[0].outputPath, 'utf8'))
console.log('records:', json.length)
console.log('first:', { title: json[0].title, permalink: json[0].permalink, slug: json[0].slug, hasBody: !!json[0].body })

// schema descriptor smoke
const schema = s.object({ title: s.string().max(99), draft: s.boolean().default(false) })
console.log('schema descriptor:', JSON.stringify(schema, null, 2))
