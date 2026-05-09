<p align="center">
  <img src="../public/logo-dark.svg" alt="dmc-transform" width="120"/>
</p>

<h1 align="center">dmc-transform</h1>

<p align="center">
  Transform pipeline and built-in transformers for the dmc compiler.
</p>

<p align="center">
  <a href="../LICENSE">MIT</a> -
  <a href="../CHANGELOG.md">Changelog</a> -
  <a href="../CONTRIBUTING.md">Contributing</a> -
  <a href="https://crates.io/crates/dmc-transform">crates.io</a> -
  <a href="https://docs.rs/dmc-transform">docs.rs</a>
</p>

<p align="center">
  <a href="https://crates.io/crates/dmc-transform"><img src="https://img.shields.io/crates/v/dmc-transform.svg" alt="crates.io"/></a>
  <a href="https://docs.rs/dmc-transform"><img src="https://docs.rs/dmc-transform/badge.svg" alt="docs.rs"/></a>
  <a href="../LICENSE"><img src="https://img.shields.io/crates/l/dmc-transform.svg" alt="MIT"/></a>
</p>

---

## Install

```sh
cargo add dmc-transform
```

## Quick start

```rust
use dmc_transform::{Pipeline, PipelineConfig};

let pipeline = Pipeline::with_defaults_for(&PipelineConfig::default());
pipeline.transform(&mut document, &meta, &mut diag);
```

## Docs

- [crates.io](https://crates.io/crates/dmc-transform)
- [docs.rs](https://docs.rs/dmc-transform)
- Full docs in the [dmc-docs/](../dmc-docs/dmc-transform/) folder (per-source-file walkthrough)

## Contributing

See [`../CONTRIBUTING.md`](../CONTRIBUTING.md).

## License

MIT. See [`../LICENSE`](../LICENSE).
