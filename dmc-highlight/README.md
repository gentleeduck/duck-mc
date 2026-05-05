<p align="center">
  <img src="../public/logo-dark.svg" alt="dmc-highlight" width="120"/>
</p>

<h1 align="center">dmc-highlight</h1>

<p align="center">
  Bundled syntect grammars and themes for dmc syntax highlighting.
</p>

<p align="center">
  <a href="../LICENSE">MIT</a> -
  <a href="../CHANGELOG.md">Changelog</a> -
  <a href="../CONTRIBUTING.md">Contributing</a> -
  <a href="https://crates.io/crates/dmc-highlight">crates.io</a> -
  <a href="https://docs.rs/dmc-highlight">docs.rs</a>
</p>

<p align="center">
  <a href="https://crates.io/crates/dmc-highlight"><img src="https://img.shields.io/crates/v/dmc-highlight.svg" alt="crates.io"/></a>
  <a href="https://docs.rs/dmc-highlight"><img src="https://docs.rs/dmc-highlight/badge.svg" alt="docs.rs"/></a>
  <a href="../LICENSE"><img src="https://img.shields.io/crates/l/dmc-highlight.svg" alt="MIT"/></a>
</p>

---

## Install

```sh
cargo add dmc-highlight
```

## Quick start

```rust
use dmc_highlight::{SyntaxBundle, Theme};

let bundle = SyntaxBundle::get();
let html = bundle.highlight("fn main() {}", "rust", Theme::CatppuccinMocha);
```

## Docs

- [crates.io](https://crates.io/crates/dmc-highlight)
- [docs.rs](https://docs.rs/dmc-highlight)
- Per-crate guide in the repo: see [`../README.md`](../README.md)

## Contributing

See [`../CONTRIBUTING.md`](../CONTRIBUTING.md).

## License

MIT. See [`../LICENSE`](../LICENSE).
