# Schema descriptors

JSON tree consumed by `compile_descriptor`. JS `s` builder emits these.

## Primitive

```json
{ "kind": "string" }
{ "kind": "string", "min": 1, "max": 99, "regex": "^[a-z]+$" }
{ "kind": "number", "min": 0, "max": 100 }
{ "kind": "boolean" }
{ "kind": "date" }
{ "kind": "path" }                     // string + filesystem-aware transform
```

## Optional / default

```json
{ "kind": "string", "optional": true }
{ "kind": "string", "default": "untitled" }
```

`optional` allows the field to be absent; `default` supplies a fallback.

## Array

```json
{ "kind": "array", "item": { "kind": "string" } }
{ "kind": "array", "item": { "kind": "string" }, "min": 1, "max": 5 }
```

## Object

```json
{
  "kind": "object",
  "fields": {
    "title": { "kind": "string" },
    "tags":  { "kind": "array", "item": { "kind": "string" }, "optional": true }
  }
}
```

`fields` is a name -> descriptor map. Unknown fields in the input
produce a warning (not an error).

## Union / literal / enum

```json
{ "kind": "union", "options": [
  { "kind": "literal", "value": "draft" },
  { "kind": "literal", "value": "published" }
]}

{ "kind": "literal", "value": "tutorial" }

{ "kind": "enum", "values": ["s", "m", "l"] }
```

## Record / map

```json
{ "kind": "record", "key": { "kind": "string" }, "value": { "kind": "number" } }
```

Maps to a JSON object with arbitrary keys.

## Markdown / MDX

```json
{ "kind": "markdown" }
{ "kind": "mdx" }
```

When present in a schema, the field's input is fed through the dmc
compile pipeline; the validated value is the rendered HTML / MDX
body. Used by velite-style schemas where a frontmatter field carries
the body.

## Composition

Descriptors nest freely:

```json
{
  "kind": "object",
  "fields": {
    "meta": {
      "kind": "object",
      "fields": {
        "author": { "kind": "string" },
        "tags":   { "kind": "array", "item": { "kind": "string" } }
      }
    }
  }
}
```

## Transforms

Some descriptors set a `transform` flag that runs post-validation:

- `path`: resolved relative to `Ctx::root`
- `markdown` / `mdx`: pipes the value through `Compiler`
- `date`: ISO -> chrono `DateTime<Utc>` (returned as ISO string)

The Rust side never invents values; transforms only reshape the input.
