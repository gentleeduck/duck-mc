<p align="center">
  <img src="../public/logo-dark.svg" alt="dmc-parser" width="120"/>
</p>

<h1 align="center">dmc-parser</h1>

<p align="center">
  Typed AST parser for the dmc MDX compiler.
</p>

<p align="center">
  <a href="../LICENSE">MIT</a> -
  <a href="../CHANGELOG.md">Changelog</a> -
  <a href="../CONTRIBUTING.md">Contributing</a> -
  <a href="https://crates.io/crates/dmc-parser">crates.io</a> -
  <a href="https://docs.rs/dmc-parser">docs.rs</a>
</p>

<p align="center">
  <a href="https://crates.io/crates/dmc-parser"><img src="https://img.shields.io/crates/v/dmc-parser.svg" alt="crates.io"/></a>
  <a href="https://docs.rs/dmc-parser"><img src="https://docs.rs/dmc-parser/badge.svg" alt="docs.rs"/></a>
  <a href="../LICENSE"><img src="https://img.shields.io/crates/l/dmc-parser.svg" alt="MIT"/></a>
</p>

---

## Install

```sh
cargo add dmc-parser
```

## Quick start

```rust
use dmc_parser::Parser;

let mut parser = Parser::new(tokens, meta, &mut diag);
let document = parser.parse();
```

## Docs

- [crates.io](https://crates.io/crates/dmc-parser)
- [docs.rs](https://docs.rs/dmc-parser)
- Per-crate guide in the repo: see [`../README.md`](../README.md)

## Contributing

See [`../CONTRIBUTING.md`](../CONTRIBUTING.md).

## License

MIT. See [`../LICENSE`](../LICENSE).
