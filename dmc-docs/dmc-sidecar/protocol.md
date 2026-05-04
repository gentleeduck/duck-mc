# Protocol

NDJSON over stdin / stdout. UTF-8. One JSON object per line. No
trailing newline at EOF (use `writeln!` / `console.log` style).

## Request

```json
{
  "id": 42,
  "markdown": "# hello\n",
  "remarkPlugins": [],
  "rehypePlugins": []
}
```

| field | type | meaning |
|-------|------|---------|
| `id` | u64 | request id; reflected on the reply for matching |
| `markdown` | string | source MDX text |
| `remarkPlugins` | array | unified remark plugin tuples |
| `rehypePlugins` | array | unified rehype plugin tuples |

Plugin tuples follow unified's shape: a string (`"remark-gfm"`) or
an array (`["rehype-pretty-code", { theme: "github-dark" }]`).

## Response (success)

```json
{
  "id": 42,
  "html": "<h1>hello</h1>"
}
```

| field | type | meaning |
|-------|------|---------|
| `id` | u64 | echoes request `id` |
| `html` | string | rendered HTML output |

## Response (failure)

```json
{
  "id": 42,
  "error": "remark-gfm: unexpected token"
}
```

| field | type | meaning |
|-------|------|---------|
| `error` | string | human-readable failure |

`run_sidecar` returns `None` on `error` replies; caller falls through
to native HTML or a no-op.

## Plugin caching

`index.mjs` keys built unified pipelines by `JSON.stringify(plugins)`.
Identical plugin specs reuse the same processor across requests, so
plugin import + setup runs once per spec per process lifetime.

## Stderr

Plugin errors and unhandled exceptions get written to stderr as plain
text. dmc-core sets `Stdio::null()` on stderr so the dev does not see
this noise; capture it manually when debugging.

## Lifecycle

- spawn: `node ${dmc_SIDECAR}/index.mjs` (env override; default
  `dmc-sidecar/index.mjs` relative to cwd)
- request loop: per-line read, process, write
- exit: closing stdin terminates the process

The pool reuses processes across requests. dmc-core never explicitly
kills them; they exit when the parent exits.
