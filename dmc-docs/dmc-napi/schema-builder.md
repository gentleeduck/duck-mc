# Schema builder (`s`)

Zod-style schema constructor. JS-side mirror of the descriptors
documented in `dmc-docs/dmc-schema/descriptors.md`.

## Import

```ts
import { s } from "@duck/md";
```

## Primitives

```ts
s.string()
s.number()
s.boolean()
s.date()
s.path()    // string + filesystem-aware transform
```

Chainable refinements:

```ts
s.string().min(1).max(99).regex(/^[a-z]+$/)
s.number().min(0).max(100).int()
s.date().min("2020-01-01")
```

## Optional / default

```ts
s.string().optional()
s.string().default("untitled")
s.number().optional()
```

## Container

```ts
s.array(s.string())
s.array(s.string()).min(1).max(5)

s.object({
  title: s.string(),
  tags: s.array(s.string()).optional(),
})

s.record(s.string(), s.number())   // map of string -> number
```

## Union / literal / enum

```ts
s.union([s.literal("draft"), s.literal("published")])
s.literal("tutorial")
s.enum(["s", "m", "l"])
```

## MDX / markdown body

```ts
s.markdown()
s.mdx()
```

When present in a schema, the field's source string is fed through
the dmc compile pipeline; the validated value is the rendered HTML
(for `markdown()`) or MDX body (for `mdx()`).

Used for collections where a frontmatter field carries the body:

```ts
collections: {
  posts: {
    name: "post",
    pattern: "posts/**/*.mdx",
    schema: s.object({
      title: s.string(),
      // body is the post's MDX content
      content: s.markdown(),
    }),
  },
},
```

## `s.path()`

```ts
s.path()                      // resolves relative to ctx.root
s.path().relative("docs")     // anchor to a sub-dir
```

Returns the resolved string. Used for slug fields:

```ts
slug: s.path()
```

The runtime sets `slug` to the file path relative to `ctx.root`,
minus the extension.

## Refinements

```ts
s.string().refine(v => !v.includes("foo"), "must not contain foo")
```

Custom predicates with error messages. Runs after all built-in
checks.

## Transforms

```ts
s.string().transform(v => v.toUpperCase())
```

Returns the transformed value as the validated record field.

## Composition

```ts
const Author = s.object({
  name: s.string(),
  email: s.string().email(),
});

const Post = s.object({
  title: s.string(),
  author: Author,
  tags: s.array(s.string()).optional(),
});
```

## Inferring types

```ts
type Post = s.infer<typeof Post>;
//   ^? { title: string; author: { name: string; email: string }; tags?: string[] }
```

`s.infer<T>` walks the schema and extracts the TS type.
`@duck/md`'s `index.d.ts` uses this so the generated record types
flow through to consumer code.

## Error format

```
schema error: title: required
schema error: tags[0]: expected string, got number
```

Path-prefixed messages identify the failing field. Engine emits a
diagnostic per failure; record falls back to raw frontmatter.

## What gets serialised to Rust

The JS builder emits a JSON descriptor (see
[`../dmc-schema/descriptors.md`](../dmc-schema/descriptors.md)).
Rust's `compile_descriptor` builds the runtime validator.

JS-side refinements / transforms run inside the napi wrapper before
the Rust engine sees the data.
