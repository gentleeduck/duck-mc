# `npm-command`

Detects fenced code blocks whose first line is `npm install ...`,
`npx create-...`, or `npx <bin>` and rewrites them as a
`<PackageManagerTabs>` JSX node carrying per-PM equivalents (npm, yarn,
pnpm, bun) as plain string attributes.

- **Source:** `dmc-transform/src/builtin/npm_command.rs`
- **Feature flag:** `npm-command`
- **Config:** none

## Detection

Triggers on these first-line shapes:

| Source | npm | yarn | pnpm | bun |
|---|---|---|---|---|
| `npm install pkg` | `npm install pkg` | `yarn add pkg` | `pnpm add pkg` | `bun add pkg` |
| `npx create-foo` | `npx create-foo` | `yarn create foo` | `pnpm create foo` | `bunx --bun create-foo` |
| `npx tool` | `npx tool` | `yarn dlx tool` | `pnpm dlx tool` | `bunx --bun tool` |

## Output JSX

```jsx
<div data-rehype-pretty-code-fragment="">
  <div data-theme="dark">
    <PackageManagerTabs npm="npm install pkg" yarn="yarn add pkg" pnpm="pnpm add pkg" bun="bun add pkg" />
  </div>
  <div data-theme="light">
    <PackageManagerTabs npm="npm install pkg" yarn="yarn add pkg" pnpm="pnpm add pkg" bun="bun add pkg" />
  </div>
</div>
```

The duplicate per-theme wrappers match velite's `rehype-pretty-code`
fragment shape so consumers' theme-toggle CSS targets both blocks
uniformly.

## Consumer contract

The consumer must register a `PackageManagerTabs` MDX component that
reads the four string props and renders a tabbed UI. dmc does no UI
rendering itself.

## Why string attrs?

Highlighted JSX subtrees would force the consumer's `<PackageManagerTabs>`
to interpret pre-tokenised content. Plain strings let the consumer
decide its own rendering (plain `<pre>`, syntax-coloured, terminal-styled,
etc).
