---
"@gentleduck/md": patch
---

Populate `BuildReport.collections` in the napi `build()` binding. The
Rust side previously returned only `{ diagnostics }`, but the JS
wrapper in `mod.js` iterates `report.collections` to run the in-process
unified pipeline whenever the user config supplies remark/rehype
plugins. With the field missing, any consumer that passes plugins
(e.g. `@gentleduck/docs`'s default config) crashed with
`TypeError: report.collections is not iterable`.

The binding now reports `{ name, outputPath, records }` for every
collection plus an `errors` array, matching the shape the JS side
expects.
