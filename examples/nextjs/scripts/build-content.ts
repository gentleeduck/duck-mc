import { build } from '@duck/md'
import config from '../duck-md.config.js'

const report = build(config)
for (const c of report.collections) {
  console.log(`✓ ${c.name} — ${c.records} records → ${c.outputPath}`)
}
