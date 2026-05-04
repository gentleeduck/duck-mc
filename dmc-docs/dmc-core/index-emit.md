# Index emission

After every collection is processed, `index::write_index` emits a
top-level `index.js` plus `index.d.ts` re-exporting each
`<name>.json` and typing the records.

Path: `dmc::engine::index::write_index`.

## Signature

```rust
pub fn write_index(
    out_dir: &Path,
    collections: &[Collection],
    format: &str,            // "esm" | "cjs"
    config_path: Option<&Path>,
) -> std::io::Result<()>;
```

## Output layout

```
.gentleduck/
|- doc.json
|- post.json
|- index.js
`- index.d.ts
```

## ESM index.js

```js
import doc from "./doc.json" assert { type: "json" };
import post from "./post.json" assert { type: "json" };
export { doc, post };
```

## CJS index.js

```js
const doc = require("./doc.json");
const post = require("./post.json");
module.exports = { doc, post };
```

## index.d.ts

```ts
import type docConfig from "<config_path>";

export type DocCollection = (typeof docConfig)["collections"]["docs"]["schema"];
export const doc: import("@duck/md").TypeOf<DocCollection>[];

export type PostCollection = (typeof docConfig)["collections"]["posts"]["schema"];
export const post: import("@duck/md").TypeOf<PostCollection>[];
```

When `config_path` is `None`, types fall back to `unknown[]`.

## Why typeof import

Pulling `typeof import(config)["collections"]` lets the user's
`s.object({...})` types flow through to the consumer's IDE without
re-declaring the schema. Editing the schema in `duck-md.config.ts`
auto-updates `.d.ts` on the next build.

## File pattern

| collection.single | output |
|-------------------|--------|
| `false` | `<name>.json` -> array of records, `Record[]` |
| `true` | `<name>.json` -> single object, `Record` |

`index.d.ts` adjusts the type to match.

## When this runs

Only after every `Collection::process` returns. Failures in one
collection do not prevent index emission for the others; the failed
collection's `<name>.json` is whatever the previous build wrote (or
absent).

## Skipping

Currently always runs. To opt out (e.g. for tests), call
`Collection::process` directly and skip `Engine::run`.

## Consumer import

```ts
import { doc, post } from "../.gentleduck";

doc.forEach(d => console.log(d.title));
//                          ^? string (from schema)
```

ESM by default. Override:

```ts
output: { format: "cjs" }
```

for CommonJS consumers.

## Cache interaction

The index file itself is not cached. It is regenerated every build
(cheap; just a small file write). The collection JSONs ARE cached
implicitly via the per-file cache, so most of the build cost stays
zero on warm rebuilds.
