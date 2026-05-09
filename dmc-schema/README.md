<p align="center">
  <img src="../public/logo-dark.svg" alt="dmc-schema" width="120"/>
</p>

<h1 align="center">dmc-schema</h1>

<p align="center">
  Velite-style schema builders for dmc collection records.
</p>

<p align="center">
  <a href="../LICENSE">MIT</a> -
  <a href="../CHANGELOG.md">Changelog</a> -
  <a href="../CONTRIBUTING.md">Contributing</a> -
  <a href="https://crates.io/crates/dmc-schema">crates.io</a> -
  <a href="https://docs.rs/dmc-schema">docs.rs</a>
</p>

<p align="center">
  <a href="https://crates.io/crates/dmc-schema"><img src="https://img.shields.io/crates/v/dmc-schema.svg" alt="crates.io"/></a>
  <a href="https://docs.rs/dmc-schema"><img src="https://docs.rs/dmc-schema/badge.svg" alt="docs.rs"/></a>
  <a href="../LICENSE"><img src="https://img.shields.io/crates/l/dmc-schema.svg" alt="MIT"/></a>
</p>

---

## Install

```sh
cargo add dmc-schema
```

## Quick start

```rust
use dmc_schema::{s, Ctx};

let schema = s::object(vec![
  ("title".into(), s::string().max(99).boxed()),
  ("draft".into(), s::default_(s::boolean().boxed(), serde_json::json!(false)).boxed()),
]);
let out = schema.parse(&value, &Ctx::empty())?;
```

## Docs

- [crates.io](https://crates.io/crates/dmc-schema)
- [docs.rs](https://docs.rs/dmc-schema)
- Full docs in the [dmc-docs/](../dmc-docs/dmc-schema/) folder (per-source-file walkthrough)

## Contributing

See [`../CONTRIBUTING.md`](../CONTRIBUTING.md).

## License

MIT. See [`../LICENSE`](../LICENSE).
