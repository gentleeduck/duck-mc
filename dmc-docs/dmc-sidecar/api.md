# dmc-sidecar API

## `index.mjs`

Entry point. Reads NDJSON from stdin, writes NDJSON to stdout. Exits
when stdin closes.

### Pipeline cache

```js
const cache = new Map();

function getProcessor(remarkPlugins, rehypePlugins) {
  const key = JSON.stringify({ r: remarkPlugins, h: rehypePlugins });
  if (cache.has(key)) return cache.get(key);
  const proc = unified()
    .use(remarkParse)
    .use(remarkPlugins)
    .use(remarkRehype, { allowDangerousHtml: true })
    .use(rehypePlugins)
    .use(rehypeStringify, { allowDangerousHtml: true });
  cache.set(key, proc);
  return proc;
}
```

Builds + caches the unified processor per plugin spec. Fresh process
imports plugins once; all subsequent same-spec requests reuse the
processor.

### Request loop

```js
process.stdin.setEncoding("utf8");
let buf = "";
for await (const chunk of process.stdin) {
  buf += chunk;
  let nl;
  while ((nl = buf.indexOf("\n")) !== -1) {
    const line = buf.slice(0, nl);
    buf = buf.slice(nl + 1);
    handle(line);
  }
}
```

Line-buffered. Each line is a request.

### `handle(line)`

```js
async function handle(line) {
  let req;
  try { req = JSON.parse(line); } catch (_) { return; }
  const { id, markdown, remarkPlugins = [], rehypePlugins = [] } = req;
  try {
    const proc = getProcessor(remarkPlugins, rehypePlugins);
    const file = await proc.process(markdown);
    process.stdout.write(JSON.stringify({ id, html: String(file) }) + "\n");
  } catch (e) {
    process.stdout.write(JSON.stringify({ id, error: String(e?.message ?? e) }) + "\n");
  }
}
```

## Env vars

| var | meaning |
|-----|---------|
| `dmc_SIDECAR` | override path to `index.mjs` (default `dmc-sidecar/index.mjs` relative to cwd) |

## Spawning manually

```bash
node dmc-sidecar/index.mjs <<EOF
{"id":1,"markdown":"# hi","remarkPlugins":[],"rehypePlugins":[]}
EOF
```

Returns one line of JSON. Send more requests by writing more lines.
