#!/usr/bin/env node
import { unified } from 'unified'
import remarkParse from 'remark-parse'
import remarkGfm from 'remark-gfm'
import remarkRehype from 'remark-rehype'
import rehypeRaw from 'rehype-raw'
import rehypeStringify from 'rehype-stringify'
import { readFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { pathToFileURL } from 'node:url'
import { resolve } from 'node:path'

const userRequire = createRequire(resolve(process.cwd(), 'package.json'))

async function main() {
  const stdin = readFileSync(0, 'utf8')
  const req = JSON.parse(stdin)
  const remarkPlugins = await loadPlugins(req.remarkPlugins ?? [])
  const rehypePlugins = await loadPlugins(req.rehypePlugins ?? [])
  const gfm = req.gfm ?? true
  const removeComments = req.removeComments ?? true

  let proc = unified().use(remarkParse)
  if (gfm) proc = proc.use(remarkGfm)
  if (removeComments) proc = proc.use(stripHtmlComments)
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

function stripHtmlComments() {
  return (tree) => {
    const walk = (node) => {
      if (!node) return
      if (Array.isArray(node.children)) {
        node.children = node.children.filter(c => c.type !== 'html' || !/^<!--[\s\S]*?-->$/.test(String(c.value ?? '')))
        node.children.forEach(walk)
      }
    }
    walk(tree)
  }
}

async function importFromUser(name) {
  let resolved
  try {
    resolved = userRequire.resolve(name)
  } catch {
    return await import(name)
  }
  return await import(pathToFileURL(resolved).href)
}

async function loadPlugins(specs) {
  const out = []
  for (const spec of specs) {
    if (typeof spec === 'string') {
      const mod = await importFromUser(spec)
      out.push([mod.default ?? mod, undefined])
    } else if (Array.isArray(spec)) {
      const [name, opts] = spec
      const mod = await importFromUser(name)
      out.push([mod.default ?? mod, opts])
    }
  }
  return out
}

main().catch(e => {
  process.stderr.write(`sidecar: ${e.stack ?? e}\n`)
  process.exit(1)
})
