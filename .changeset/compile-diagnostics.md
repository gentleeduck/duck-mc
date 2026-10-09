---
"@gentleduck/md": patch
---

`compile` and `compileMany` now return the diagnostics the compile produced, on `diagnostics`, and `PrettyCodeBundledTheme` lists the themes that are actually bundled.

Both halves of one silent failure. `compile` built a diagnostic engine, handed it to the pipeline and dropped it, so nothing the compile reported reached the caller. Meanwhile the theme type advertised 15 names that are not in the bundle — `github-light` and `github-dark` among them — and omitted the 15 that are. An unbundled name is a warning and a fallback, not an error, and the type's `(string & {})` escape hatch means a wrong name typechecks. So a light/dark pair of unbundled names compiled to one theme twice — a dark mode rendering in light colors — and the warning explaining why was discarded.

`compileMany` now uses one engine per source; a shared one gave every later output the diagnostics of the sources before it.
