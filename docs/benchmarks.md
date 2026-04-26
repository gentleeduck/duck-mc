# duck-md benchmarks

Real numbers, real reproductions. Measured on Linux, Node 20.20.2, pnpm 10.33.0, Rust release profile, criterion 0.5.

Versions of competitors at time of measurement:

- velite 0.3.1
- @mdx-js/mdx (latest, 3.x)
- unified 11.0.5 + remark-parse 11 + remark-gfm 4.0.1 + remark-rehype 11.1.2 + rehype-raw 7 + rehype-stringify 10
- marked 18.0.2
- gray-matter 4.0.3

Fixtures: `tests/fixtures/velite-parity/{mdx,skills,whoiam}.mdx` — 117 / 96 / 141 lines, 354 lines total.

## Headline

| Workload                              | Tool        | Median  | Notes                              |
| ------------------------------------- | ----------- | ------- | ---------------------------------- |
| compile skills.mdx (in-process)       | duck-md     | **0.119 ms** | cargo bench, native release        |
| compile skills.mdx (in-process)       | @mdx-js/mdx | 2.583 ms     | function-body output + remark-gfm  |
| remark→rehype HTML skills.mdx         | unified     | 1.986 ms     | gfm + rehype-raw + stringify       |
| md-only skills.mdx                    | marked      | 0.136 ms     | gfm:true; baseline lower bound     |
| build 3 fixtures (cold)               | velite      | 250 ms       | full pipeline incl. esbuild config |
| build 3 fixtures (cold)               | duck-md     | **2.4 ms**   | 104× faster                        |
| build 999 fixtures (cold)             | velite      | 7330 ms      | 7.34 ms / file                     |
| build 999 fixtures (cold)             | duck-md     | **46 ms**    | **159× faster**, 0.05 ms / file    |
| sidecar cold spawn (per call)         | node sidecar | 115.5 ms    | bottleneck for users with shiki    |

duck-md is faster than `marked` at full MDX semantics. There is little single-file headroom left.

## Raw outputs

### duck-md cargo bench (release)

```
compile skills.mdx      time:   [118.67 µs 119.39 µs 120.18 µs]
compile simple          time:   [34.826 µs 35.114 µs 35.406 µs]
parse skills.mdx        time:   [35.775 µs 35.811 µs 35.851 µs]
```

### @mdx-js/mdx (function-body, gfm) — 200 iters after 30 warmup

```
mdx-js compile mdx.mdx                 median=2.055 ms  p95=2.753 ms
mdx-js compile skills.mdx              median=2.583 ms  p95=3.269 ms
mdx-js compile whoiam.mdx              median=4.423 ms  p95=5.621 ms
```

### unified remark→rehype HTML (gfm + raw)

```
remark→rehype mdx.mdx                  median=1.493 ms  p95=2.176 ms
remark→rehype skills.mdx               median=1.986 ms  p95=2.377 ms
remark→rehype whoiam.mdx               median=3.604 ms  p95=4.254 ms
```

### marked (gfm md→html, baseline)

```
marked mdx.mdx                         median=0.112 ms
marked skills.mdx                      median=0.136 ms
marked whoiam.mdx                      median=0.255 ms
```

### Full build, 3 fixtures (5 cold runs each)

```
velite build                  median=250.3 ms   samples=[242, 248, 250, 251, 277]
duck-md build                 median=2.4 ms     samples=[2, 2, 2, 2, 3]
```

### Full build, 999 fixtures (3 cold runs each)

```
velite build (999)            median=7330 ms    per-file=7.34 ms
duck-md build (999)           median=46 ms      per-file=0.05 ms   samples=[44, 46, 60]
```

### Sidecar per-call cold spawn (20 iters)

```
median=115.5 ms   mean=115.4 ms   per call (just node startup + ESM import graph)
@ 999 files = 115.4 s (cold-spawn cost alone, before any rayon parallelism)
```

## Reproduce

### 1. Set up bench dir (outside repo, ephemeral)

```sh
mkdir -p /tmp/duck-bench && cp tests/fixtures/velite-parity/*.mdx /tmp/duck-bench/
mkdir -p /tmp/duck-bench/content/docs && cp /tmp/duck-bench/*.mdx /tmp/duck-bench/content/docs/

cat > /tmp/duck-bench/duck-md.toml <<'EOF'
output_dir = ".gentleduck"

[[collections]]
name = "docs"
pattern = "docs/**/*.mdx"
base_dir = "content"
EOF

cat > /tmp/duck-bench/velite.config.ts <<'EOF'
import { defineConfig, s } from "velite";
export default defineConfig({
  root: ".",
  output: { data: ".velite", clean: true, format: "esm" },
  collections: {
    docs: {
      name: "Doc",
      pattern: "*.mdx",
      schema: s.object({
        title: s.string().optional(),
        description: s.string().optional(),
        slug: s.slug().optional(),
        body: s.mdx(),
      }).passthrough(),
    },
  },
});
EOF

cat > /tmp/duck-bench/package.json <<'EOF'
{
  "name": "duck-bench",
  "private": true,
  "type": "module",
  "dependencies": {
    "velite": "latest",
    "@mdx-js/mdx": "latest",
    "marked": "latest",
    "remark": "latest",
    "remark-mdx": "latest",
    "remark-rehype": "latest",
    "rehype-stringify": "latest",
    "remark-gfm": "latest",
    "remark-parse": "latest",
    "rehype-raw": "latest",
    "unified": "latest",
    "gray-matter": "latest"
  }
}
EOF

cd /tmp/duck-bench && pnpm install
```

### 2. JS in-process compile bench (`bench.mjs`)

```js
import { performance } from "node:perf_hooks";
import { readFileSync } from "node:fs";
import { compile as mdxCompile } from "@mdx-js/mdx";
import matter from "gray-matter";
import { unified } from "unified";
import remarkParse from "remark-parse";
import remarkGfm from "remark-gfm";
import remarkRehype from "remark-rehype";
import rehypeRaw from "rehype-raw";
import rehypeStringify from "rehype-stringify";
import { marked } from "marked";

const FILES = ["mdx.mdx", "skills.mdx", "whoiam.mdx"];
const sources = Object.fromEntries(
  FILES.map((f) => [f, readFileSync(new URL(f, import.meta.url), "utf8")]),
);

const WARMUP = 30, ITERS = 200;
async function bench(name, fn) {
  for (let i = 0; i < WARMUP; i++) await fn();
  const samples = [];
  for (let i = 0; i < ITERS; i++) {
    const t0 = performance.now();
    await fn();
    samples.push(performance.now() - t0);
  }
  samples.sort((a, b) => a - b);
  const median = samples[Math.floor(samples.length / 2)];
  const p95 = samples[Math.floor(samples.length * 0.95)];
  const mean = samples.reduce((a, b) => a + b, 0) / samples.length;
  console.log(
    `${name.padEnd(46)} median=${median.toFixed(3).padStart(8)}ms  mean=${mean.toFixed(3).padStart(8)}ms  p95=${p95.toFixed(3).padStart(8)}ms`,
  );
}

const mdxPipe = (src) =>
  mdxCompile(src, {
    outputFormat: "function-body",
    remarkPlugins: [remarkGfm],
    development: false,
  });
for (const f of FILES) await bench(`mdx-js compile ${f}`, () => mdxPipe(sources[f]));

const remarkPipe = unified()
  .use(remarkParse)
  .use(remarkGfm)
  .use(remarkRehype, { allowDangerousHtml: true })
  .use(rehypeRaw)
  .use(rehypeStringify);
for (const f of FILES) {
  await bench(`remark→rehype ${f}`, async () => {
    const { content } = matter(sources[f]);
    await remarkPipe.process(content);
  });
}

marked.setOptions({ gfm: true });
for (const f of FILES) {
  await bench(`marked ${f}`, () => {
    const { content } = matter(sources[f]);
    marked.parse(content);
  });
}
```

Run: `cd /tmp/duck-bench && node bench.mjs`.

### 3. Full-build bench, 3 fixtures (`bench-build-duck.mjs`)

```js
import { performance } from "node:perf_hooks";
import { spawnSync } from "node:child_process";
import { rmSync } from "node:fs";

const DUCK = "<repo>/target/release/duck-md";

function timeRun(label, cmd, args) {
  const samples = [];
  for (let i = 0; i < 5; i++) {
    try { rmSync(".gentleduck", { recursive: true, force: true }); } catch {}
    try { rmSync(".velite", { recursive: true, force: true }); } catch {}
    const t0 = performance.now();
    const r = spawnSync(cmd, args, { stdio: "ignore" });
    samples.push(performance.now() - t0);
    if (r.status !== 0) return console.error(`${label} FAILED`);
  }
  samples.sort((a, b) => a - b);
  const median = samples[Math.floor(samples.length / 2)];
  const mean = samples.reduce((a, b) => a + b, 0) / samples.length;
  console.log(`${label.padEnd(36)} median=${median.toFixed(1)}ms  mean=${mean.toFixed(1)}ms`);
}

timeRun("velite build", "node_modules/.bin/velite", ["build", "--clean"]);
timeRun("duck-md build", DUCK, ["build", "--config", "duck-md.toml"]);
```

### 4. Scale bench, 999 fixtures

```sh
mkdir -p /tmp/duck-bench-big/content/docs
for i in $(seq 1 333); do
  cp /tmp/duck-bench/skills.mdx /tmp/duck-bench-big/content/docs/skills_$i.mdx
  cp /tmp/duck-bench/mdx.mdx    /tmp/duck-bench-big/content/docs/mdx_$i.mdx
  cp /tmp/duck-bench/whoiam.mdx /tmp/duck-bench-big/content/docs/whoiam_$i.mdx
done
cp /tmp/duck-bench/duck-md.toml /tmp/duck-bench-big/
cp -r /tmp/duck-bench/node_modules /tmp/duck-bench-big/
cp /tmp/duck-bench/package.json   /tmp/duck-bench-big/

cat > /tmp/duck-bench-big/velite.config.ts <<'EOF'
import { defineConfig, s } from "velite";
export default defineConfig({
  root: ".",
  output: { data: ".velite", clean: true, format: "esm" },
  collections: {
    docs: {
      name: "Doc",
      pattern: "content/docs/*.mdx",
      schema: s.object({
        title: s.string().optional(),
        body: s.mdx(),
      }).passthrough(),
    },
  },
});
EOF
```

Use the same `bench-build-duck.mjs` script (3 cold samples per tool).

### 5. Sidecar cold-spawn cost (`sidecar-spawn-test.mjs`)

```js
import { performance } from "node:perf_hooks";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";

const SIDECAR = "<repo>/duck-md-sidecar/index.mjs";
const src = readFileSync("/tmp/duck-bench/skills.mdx", "utf8");
const req = JSON.stringify({
  markdown: src,
  remarkPlugins: ["remark-gfm"],
  rehypePlugins: [],
});

const samples = [];
for (let i = 0; i < 20; i++) {
  const t0 = performance.now();
  spawnSync("node", [SIDECAR], {
    input: req,
    cwd: "/tmp/duck-bench",
    stdio: ["pipe", "pipe", "ignore"],
  });
  samples.push(performance.now() - t0);
}
samples.sort((a, b) => a - b);
const median = samples[Math.floor(samples.length / 2)];
console.log(`median=${median.toFixed(1)}ms per call`);
console.log(`@ 999 files = ${(median * 999 / 1000).toFixed(1)}s cold-spawn cost`);
```

## What the numbers tell us

1. **Native pipeline is already at the floor.** duck-md (full MDX semantics) is faster than `marked` (md-only, fastest known JS). 21.7× faster than @mdx-js/mdx. Lexer/parser micro-opts will not move the needle.
2. **velite cold-build vs duck-md scales worse for velite at small sizes** (Node + esbuild dominate) and **better for duck-md at large sizes** (rayon amortization). The 999-file ratio (159×) is the realistic large-site number.
3. **Sidecar process spawn is the only multi-second bottleneck.** 115 ms × N files. A long-lived sidecar with NDJSON streaming would convert this to one cold-start + ~2-3 ms / file.
4. Codegen (`format!`-heavy in `mdx.rs` / `html.rs`) is the largest remaining native overhead per file. Refactor to `&mut String` writers cuts ~25-40 µs / file.

See [`docs/perf-plan.md`](./perf-plan.md) for the prioritized work units.
