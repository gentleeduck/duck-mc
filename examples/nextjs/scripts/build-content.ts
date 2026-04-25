import { build } from '@duck/md'
import config from '../duck-md.config.js'

async function main() {
  const report = await build(config)
  for (const c of report.collections) {
    console.log(`✓ ${c.name} — ${c.records} records → ${c.outputPath}`)
  }
}
main().catch((e) => { console.error(e); process.exit(1) })
