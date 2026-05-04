# dmc-schema examples

## Compile a descriptor and validate

```rust
use serde_json::json;
use dmc_schema::compile_descriptor;

let descriptor = json!({
    "kind": "object",
    "fields": {
        "title": { "kind": "string", "max": 99 },
        "tags":  { "kind": "array", "item": { "kind": "string" }, "optional": true }
    }
});

let schema = compile_descriptor(&descriptor).expect("compile");
let _ = schema;
```

## Run validation in a transformer

```rust
use std::path::PathBuf;
use serde_json::json;
use dmc_schema::{compile_descriptor, Ctx, ContextMeta};

let schema = compile_descriptor(&json!({
    "kind": "object",
    "fields": {
        "title": { "kind": "string" },
        "draft": { "kind": "boolean", "optional": true }
    }
})).unwrap();

let fm = json!({ "title": "Hello", "draft": false });
let ctx = Ctx {
    path: PathBuf::from("posts/hello.mdx"),
    root: PathBuf::from("content"),
    meta: ContextMeta::default(),
};

let validated = schema.parse(&fm, &ctx).expect("valid");
assert_eq!(validated["title"], "Hello");
```

## Failing input

```rust
let bad = json!({ "title": 42 });
let err = schema.parse(&bad, &ctx).unwrap_err();
assert!(err.contains("title"));
```

The error message names the failing path. Engine surfaces it as a
diagnostic.

## In a velite-style config

```ts
import { defineConfig, s } from "@gentleduck/md";

export default defineConfig({
  collections: {
    posts: {
      name: "post",
      pattern: "posts/**/*.mdx",
      schema: s.object({
        title: s.string().max(99),
        slug: s.path(),
        date: s.date(),
        excerpt: s.string().optional(),
      }),
    },
  },
});
```

`s.object(...).serialise()` produces the descriptor JSON; the engine
calls `compile_descriptor` once per collection.
