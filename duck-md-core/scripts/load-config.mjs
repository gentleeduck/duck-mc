const target = process.argv[2]
if (!target) {
  process.stderr.write('load-config: missing config path\n')
  process.exit(2)
}

const m = await import(target)
const cfg = m.default ?? m

const root = cfg.root ?? '.'
const outputDir = cfg.output?.data ?? '.gentleduck'
const collections = Object.entries(cfg.collections ?? {}).map(([key, c]) => ({
  name: c.name ?? key,
  pattern: Array.isArray(c.pattern) ? c.pattern[0] : c.pattern,
  base_dir: c.baseDir ?? root,
  schema: c.schema ?? null,
  single: !!c.single,
}))

const adapted = {
  output_dir: outputDir,
  root,
  strict: !!cfg.strict,
  clean: !!cfg.output?.clean,
  output_assets: cfg.output?.assets ?? null,
  output_base: cfg.output?.base ?? null,
  output_name: cfg.output?.name ?? null,
  output_format: cfg.output?.format ?? null,
  collections,
}

process.stdout.write(JSON.stringify(adapted))
