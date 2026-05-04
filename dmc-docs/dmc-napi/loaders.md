# Custom loaders

Plug a non-MDX file type into a collection. Loader takes a file path
and returns parsed `data` for the engine to feed through schema
validation.

## API

```ts
import { defineLoader } from "@duck/md";

export type Loader = {
  test: RegExp | string;
  load(input: { path: string; value: string }): Promise<Result> | Result;
};

export type Result = {
  data: unknown;
  // optional extra fields routed to the record
  body?: string;
  excerpt?: string;
};
```

Path: `dmc_napi::Loader`. Configured via `UserConfig.loaders`.

## Example: YAML loader

```ts
import yaml from "js-yaml";
import { defineConfig, defineLoader, s } from "@duck/md";

const yamlLoader = defineLoader({
  test: /\.ya?ml$/,
  load({ path, value }) {
    return { data: yaml.load(value) };
  },
});

export default defineConfig({
  loaders: [yamlLoader],
  collections: {
    settings: {
      name: "setting",
      pattern: "settings/*.yaml",
      schema: s.object({
        name: s.string(),
        version: s.string(),
      }),
    },
  },
});
```

The collection now matches `.yaml` files; `yamlLoader.load` parses
each into a `data` object, then schema validation runs on it.

## Multiple loaders

```ts
loaders: [yamlLoader, jsonLoader, csvLoader]
```

First match wins (`.test` evaluated in order). MDX files always go
through the built-in MDX path; loaders are tried only when the
extension is not `.mdx` / `.md`.

## Async

```ts
defineLoader({
  test: /\.toml$/,
  async load({ path, value }) {
    const toml = await import("@iarna/toml");
    return { data: toml.parse(value) };
  },
});
```

Loaders may be async. The engine awaits each per-file load before
running schema validation.

## Returning extra fields

```ts
defineLoader({
  test: /\.txt$/,
  load({ path, value }) {
    return {
      data: { title: extractTitle(value) },
      body: value,
      excerpt: value.slice(0, 200),
    };
  },
});
```

`body` and `excerpt` flow through to the final record alongside
schema-validated `data`.

## Use cases

- YAML / TOML / JSON config collections.
- Custom note-taking formats (Org-mode, AsciiDoc).
- Front-of-house data (CSV product lists, RSS feeds).
- Wrapping a non-MDX renderer (return `body` as pre-rendered HTML).

## Built-in MDX loader

The MDX path is implicit; you do not register it. Files matching
`.mdx` / `.md` always go through the dmc engine compile pipeline,
not your loaders. To handle MDX with custom semantics, write a
remark plugin and pass via `markdown.remarkPlugins` (sidecar) or
contribute a Rust transformer.

## Errors

Throwing inside `load`:

```ts
throw new Error("malformed yaml at " + path);
```

is caught by the engine; the error is added to `report.errors` and
the file is skipped. Other files continue to compile.
