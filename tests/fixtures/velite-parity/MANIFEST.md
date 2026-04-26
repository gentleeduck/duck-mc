# Velite parity fixtures

These MDX fixtures were copied verbatim from the velite-powered docs site so
that `duck-md` can be smoke-tested against real input.

## Sources

| Fixture      | Source path                                  | Lines |
| ------------ | -------------------------------------------- | ----- |
| `mdx.mdx`    | `@duck-ui/apps/duck/content/docs/mdx.mdx`    | 117   |
| `skills.mdx` | `@duck-ui/apps/duck/content/docs/skills.mdx` | 96    |
| `whoiam.mdx` | `@duck-ui/apps/duck/content/docs/whoiam.mdx` | 141   |

Absolute origin (on the machine the fixtures were vendored from):
`/run/media/wildduck/duck/wildduck/@duck/@duck-ui/apps/duck/content/docs/`

## Notes

- All three files have YAML frontmatter with at least a `title` field.
- All three files have `##`/`###` headings, so a TOC should be produced.
- `mdx.mdx` and `whoiam.mdx` use JSX components (`<Callout>`, `<a>`, etc.).
- Fixtures with a `.known_fail.mdx` suffix (none yet) are expected to panic
  the parser and are skipped by the parity tests.
