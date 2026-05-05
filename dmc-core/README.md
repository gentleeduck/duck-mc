<p align="center">
  <img src="../public/logo-dark.svg" alt="dmc-core" width="120"/>
</p>

<h1 align="center">dmc-core</h1>

<p align="center">
  Engine, CLI, watch mode, and collection builds for the dmc MDX compiler.
</p>

<p align="center">
  <a href="../LICENSE">MIT</a> -
  <a href="../CHANGELOG.md">Changelog</a> -
  <a href="../CONTRIBUTING.md">Contributing</a> -
  <a href="https://crates.io/crates/dmc-core">crates.io</a> -
  <a href="https://docs.rs/dmc-core">docs.rs</a>
</p>

<p align="center">
  <a href="https://crates.io/crates/dmc-core"><img src="https://img.shields.io/crates/v/dmc-core.svg" alt="crates.io"/></a>
  <a href="https://docs.rs/dmc-core"><img src="https://docs.rs/dmc-core/badge.svg" alt="docs.rs"/></a>
  <a href="../LICENSE"><img src="https://img.shields.io/crates/l/dmc-core.svg" alt="MIT"/></a>
</p>

---

## Install

```sh
cargo add dmc-core
```

## Quick start

```rust
use dmc::engine::{Engine, EngineConfig};

let cfg = EngineConfig::from_file("dmc.config.toml")?;
Engine::run(&cfg, None, &mut diag)?;
```

## Docs

- [crates.io](https://crates.io/crates/dmc-core)
- [docs.rs](https://docs.rs/dmc-core)
- Per-crate guide in the repo: see [`../README.md`](../README.md)

## Contributing

See [`../CONTRIBUTING.md`](../CONTRIBUTING.md).

## License

MIT. See [`../LICENSE`](../LICENSE).
