import config from '../duck-md.config.ts'
import { build } from '@duck/md'

console.log('typeof first remark plugin:', typeof config.mdx?.remarkPlugins?.[0])
console.log('first remark plugin:', config.mdx?.remarkPlugins?.[0])

const rep = await build(config)
console.log('errors:', rep.errors)
console.log('records:', rep.collections.map(c => ({ name: c.name, records: c.records })))
const fs = await import('node:fs')
const j = JSON.parse(fs.readFileSync(rep.collections[0].outputPath, 'utf8'))
console.log('first.html starts:', JSON.stringify(j[0].html?.slice(0, 200)))
