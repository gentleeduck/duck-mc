import duck from './mod.js'
const { compile, build, defineConfig } = duck
import fs from 'node:fs'
import path from 'node:path'
import os from 'node:os'

// 1) compile() — single source string
const out = compile(`---\ntitle: Hi\n---\n\n# Hello **world**\n\n- a\n- b\n`)
console.log('compile.title:', out.frontmatter?.title)
console.log('compile.metadata:', out.metadata)
console.log('compile.toc:', JSON.stringify(out.toc))
console.log('compile.html:', out.html.slice(0, 80) + '…')
console.log('compile.body chars:', out.body.length)

// 2) build() — full engine, velite-shape JSON output
const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'duck-md-napi-'))
fs.mkdirSync(path.join(dir, 'docs'), { recursive: true })
fs.writeFileSync(path.join(dir, 'docs', 'a.mdx'), `---\ntitle: A\n---\n# Alpha\n`)
fs.writeFileSync(path.join(dir, 'docs', 'b.mdx'), `---\ntitle: B\n---\n# Beta\n`)

const cfg = defineConfig({
  outputDir: path.join(dir, '.gentleduck'),
  collections: [
    { name: 'docs', pattern: 'docs/**/*.mdx', baseDir: dir },
  ],
})
const rep = build(cfg)
console.log('build.report:', JSON.stringify(rep, null, 2))

const json = JSON.parse(fs.readFileSync(path.join(dir, '.gentleduck', 'docs.json'), 'utf8'))
console.log('records:', json.length)
console.log('first:', { title: json[0].title, permalink: json[0].permalink, slug: json[0].slug, hasBody: !!json[0].body })
console.log('camelCase metadata:', json[0].metadata)
