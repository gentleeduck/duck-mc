# NpmCommand

Generates package-manager-tabbed JSX from a single `npm` command. So
authors write `npm i react` once and get a tab UI showing the
equivalent for `pnpm`, `yarn`, `bun`.

## Feature flag

`npm-command` (default on).

## Input

Code blocks tagged `lang=npm`:

````md
```npm
i react react-dom
```
````

`Node::CodeBlock { lang: Some("npm"), value, .. }`.

## Output

A `<PackageManagerTabs>` JSX element with one tab per package manager:

```html
<PackageManagerTabs>
  <Tab name="npm"><pre><code>npm i react react-dom</code></pre></Tab>
  <Tab name="pnpm"><pre><code>pnpm add react react-dom</code></pre></Tab>
  <Tab name="yarn"><pre><code>yarn add react react-dom</code></pre></Tab>
  <Tab name="bun"><pre><code>bun add react react-dom</code></pre></Tab>
</PackageManagerTabs>
```

The element name `PackageManagerTabs` matches a recognised raw-HTML
emit pattern in `HtmlEmitter` (renders the tab UI).

## Translation rules

| npm | pnpm | yarn | bun |
|-----|------|------|-----|
| `i pkg` / `install pkg` | `add pkg` | `add pkg` | `add pkg` |
| `i -D pkg` | `add -D pkg` | `add -D pkg` | `add -D pkg` |
| `i -g pkg` | `add -g pkg` | `global add pkg` | `add -g pkg` |
| `run script` | `run script` | `run script` | `run script` |
| `npx X` | `pnpm dlx X` | `yarn dlx X` | `bunx X` |
| `npm create X` | `pnpm create X` | `yarn create X` | `bun create X` |

Multiline commands are translated line by line.

## API

```rust
pub struct NpmCommand;

impl Transformer for NpmCommand {
    fn name(&self) -> &str { "npm-command" }
}
```

Path: `dmc_transform::NpmCommand`.

## Example

Source:

````md
```npm
i react

run dev
```
````

Renders as a tabbed UI; the `pnpm` tab shows:

```
pnpm add react

pnpm run dev
```

## Why a transformer

Doc sites previously needed manual tab markup per command. This pass
keeps the source single-line; the tab UI is generated.

## Composing

Runs before `PrettyCode` so the inner `<pre><code>` blocks inside
each tab still get syntax-highlighted.
