---
"@gentleduck/md": minor
---

`compile` and `compileMany` now take compile options, so a caller can pass `prettyCode` and get highlighted HTML back instead of highlighting in the browser.

Without this the two string-compiling entry points were fixed to the defaults: whatever theme the defaults pick, and no way to ask for two. A site with a light and a dark mode therefore could not be served from one compile, which is the case `multiThemeStrategy: "css-vars"` exists for.

```ts
const { html } = compile(source, {
  prettyCode: { theme: { light: "github-light", dark: "github-dark" }, multiThemeStrategy: "css-vars" },
})
```

`MultiThemeStrategy` is now re-exported from the crate root as well — `PrettyCodeOptions.multi_theme_strategy` is public, so the type naming it should be too.
