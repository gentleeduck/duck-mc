#!/usr/bin/env node
import { unified } from 'unified'
import remarkParse from 'remark-parse'
import remarkRehype from 'remark-rehype'
import rehypeRaw from 'rehype-raw'
import rehypeStringify from 'rehype-stringify'
import { readFileSync } from 'node:fs'

async function main() {
  const stdin = readFileSync(0, 'utf8')
  const req = JSON.parse(stdin)
  const remarkPlugins = await loadPlugins(req.remarkPlugins ?? [])
  const rehypePlugins = await loadPlugins(req.rehypePlugins ?? [])

  let proc = unified().use(remarkParse)
  for (const [plugin, opts] of remarkPlugins) proc = proc.use(plugin, opts)
  proc = proc.use(remarkRehype, { allowDangerousHtml: true }).use(rehypeRaw)
  for (const [plugin, opts] of rehypePlugins) proc = proc.use(plugin, opts)
  proc = proc.use(rehypeStringify, { allowDangerousHtml: true })

  const file = await proc.process(req.markdown)
  const out = { html: String(file), messages: file.messages.map(m => ({
    reason: m.reason, line: m.line, column: m.column,
  })) }
  process.stdout.write(JSON.stringify(out))
}

async function loadPlugins(specs) {
  const out = []
  for (const spec of specs) {
    if (typeof spec === 'string') {
      const mod = await import(spec)
      out.push([mod.default ?? mod, undefined])
    } else if (Array.isArray(spec)) {
      const [name, opts] = spec
      const mod = await import(name)
      out.push([mod.default ?? mod, opts])
    }
  }
  return out
}

main().catch(e => {
  process.stderr.write(`sidecar: ${e.stack ?? e}\n`)
  process.exit(1)
})
