# dmc-sidecar examples

## Manual spawn (debugging)

```bash
node dmc-sidecar/index.mjs
# stdin awaits one JSON object per line
# paste:
{"id":1,"markdown":"# hello","remarkPlugins":[],"rehypePlugins":[]}
# returns:
{"id":1,"html":"<h1>hello</h1>"}
```

Useful for poking at a plugin chain without invoking the full dmc
build.

## Single foreign plugin

```bash
{"id":2,"markdown":"a~b~c","remarkPlugins":[["remark-frontmatter"]],"rehypePlugins":[]}
```

Plugin tuples mirror unified's shape: bare string or
`[name, options]` array.

## Multiple plugins

```bash
{
  "id": 3,
  "markdown": "# hi\n\n[ ] todo\n",
  "remarkPlugins": [
    "remark-gfm",
    ["remark-frontmatter", { "type": "yaml", "marker": "-" }]
  ],
  "rehypePlugins": [
    ["rehype-external-links", { "rel": ["nofollow"] }]
  ]
}
```

`remark-gfm` is stripped before dispatch when the `pretty-code`
feature is on (dmc parser handles GFM natively); the example shows
the raw payload shape.

## Inline test from Node

```js
import { spawn } from "node:child_process";

const child = spawn("node", ["dmc-sidecar/index.mjs"], {
  stdio: ["pipe", "pipe", "inherit"],
});

child.stdin.write(JSON.stringify({
  id: 1,
  markdown: "**bold**",
  remarkPlugins: [],
  rehypePlugins: [],
}) + "\n");

let buf = "";
for await (const chunk of child.stdout) {
  buf += chunk;
  const nl = buf.indexOf("\n");
  if (nl !== -1) {
    console.log(JSON.parse(buf.slice(0, nl)));
    child.stdin.end();
    break;
  }
}
```

Demonstrates the full client path: spawn -> write request -> read
response -> close.
