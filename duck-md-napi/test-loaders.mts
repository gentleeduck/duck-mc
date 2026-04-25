import duck from './mod.js'
import { mkdtempSync, mkdirSync, writeFileSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import { tmpdir } from 'node:os'

const { build, defineConfig, defineLoader } = duck

const dir = mkdtempSync(join(tmpdir(), 'duck-loader-'))
mkdirSync(join(dir, 'data'), { recursive: true })
writeFileSync(join(dir, 'data', 'person.toml'), `name = "Ada"\nage = 36\n`)
writeFileSync(join(dir, 'data', 'person2.toml'), `name = "Hopper"\nage = 85\n`)

const tomlLoader = defineLoader({
  test: /\.toml$/,
  load: ({ value }: { value: string }) => {
    const obj: Record<string, unknown> = {}
    for (const line of value.split('\n')) {
      const m = line.match(/^\s*(\w+)\s*=\s*(.+)$/)
      if (!m) continue
      const v = m[2].trim()
      obj[m[1]] = v.startsWith('"') ? v.slice(1, -1) : Number(v)
    }
    return obj
  },
})

const cfg = defineConfig({
  root: dir,
  output: { data: join(dir, '.gentleduck'), clean: true },
  loaders: [tomlLoader],
  collections: {
    people: {
      name: 'person',
      pattern: 'data/**/*.toml',
      baseDir: dir,
    },
  },
})

const rep = await build(cfg)
console.log('records:', rep.collections[0].records)
const records = JSON.parse(readFileSync(rep.collections[0].outputPath, 'utf8'))
console.log('output:', JSON.stringify(records, null, 2))
