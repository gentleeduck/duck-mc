---
"@gentleduck/md": patch
---

Frontmatter whose YAML does not parse now warns, PW002, at the line and column in the file where YAML stopped, instead of compiling the page as if it had no frontmatter.

The page used to lose every field without a word. Its record came out with no title or description, and the collection schema, which skips a page with no frontmatter, said nothing either. An unquoted `: ` inside a value is enough, as in `description: Run it: then check`. On a 548-page docs site that was 25 pages, found only because their search entries had no title.

The page still builds. It stays out of the cache while the warning stands, so every build repeats the warning until the YAML is fixed.

The first build after upgrading compiles every page once instead of reading the cache. A page an earlier version cached may be one whose YAML did not parse, cached then as clean, and reading it would skip the warning.
