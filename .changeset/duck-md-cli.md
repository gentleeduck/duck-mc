---
"@gentleduck/md": minor
---

Ship a `duck-md` CLI binary (build / dev / info / clean) so Node projects no longer need a hand-written runner around `build(config)`. `dev` mode watches the resolved root with chokidar and rebuilds on debounced FS events; `build` is a single-shot equivalent and also emits the velite-shaped `<output>/index.{js,d.ts}` so `import { duckUi } from '../.gentleduck'` keeps resolving.

Also fix two long-standing publish issues:

- The post-build step now copies the napi-rs shim to `index.cjs`; `mod.js` prefers it because `createRequire` refuses to load `.js` as CJS from an ESM caller in a `"type": "module"` package.
- Frontmatter `$` runs (e.g. `description: $subject.id, $resource.attributes.ownerId`) are no longer pair-matched as math spans -- the math preprocessor skips YAML / TOML frontmatter blocks at byte 0.
