<p align="center">
  <img src="../../public/logo-dark.svg" alt="dmc-web" width="120"/>
</p>

<h1 align="center">dmc-web</h1>

<p align="center">
  Vite + React demo rendering the compiled MDX body string at runtime.
</p>

<p align="center">
  <a href="../../LICENSE">MIT</a> -
  <a href="../../README.md">repo</a>
</p>

---

## Run

```sh
pnpm --filter dmc-web dev
```

Vite-based; no Next.js. Imports the compiled module from
`.gentleduck/` and runs it via @mdx-js/mdx runtime.
