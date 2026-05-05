<p align="center">
  <img src="../../public/logo-dark.svg" alt="dmc-nextjs" width="120"/>
</p>

<h1 align="center">dmc-nextjs</h1>

<p align="center">
  Next.js App Router demo. Renders MDX through @gentleduck/md natively.
</p>

<p align="center">
  <a href="../../LICENSE">MIT</a> -
  <a href="../../README.md">repo</a>
</p>

---

## Run

```sh
pnpm --filter dmc-nextjs dev
```

## Layout

- `content/docs/*.mdx` content collection
- `scripts/build-content.ts` runs @gentleduck/md before next dev/build
- `app/` App Router pages reading from `.gentleduck/`

## Compare

Side-by-side with the velite version: [`../nextjs-velite`](../nextjs-velite).
For numbers see [`../../duck-benchmarks`](../../duck-benchmarks).
