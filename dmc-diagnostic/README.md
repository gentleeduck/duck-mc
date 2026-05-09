<p align="center">
  <img src="../public/logo-dark.svg" alt="dmc-diagnostic" width="120"/>
</p>

<h1 align="center">dmc-diagnostic</h1>

<p align="center">
  Shared diagnostic codes and source metadata for the dmc MDX compiler.
</p>

<p align="center">
  <a href="../LICENSE">MIT</a> -
  <a href="../CHANGELOG.md">Changelog</a> -
  <a href="../CONTRIBUTING.md">Contributing</a> -
  <a href="https://crates.io/crates/dmc-diagnostic">crates.io</a> -
  <a href="https://docs.rs/dmc-diagnostic">docs.rs</a>
</p>

<p align="center">
  <a href="https://crates.io/crates/dmc-diagnostic"><img src="https://img.shields.io/crates/v/dmc-diagnostic.svg" alt="crates.io"/></a>
  <a href="https://docs.rs/dmc-diagnostic"><img src="https://docs.rs/dmc-diagnostic/badge.svg" alt="docs.rs"/></a>
  <a href="../LICENSE"><img src="https://img.shields.io/crates/l/dmc-diagnostic.svg" alt="MIT"/></a>
</p>

---

## Install

```sh
cargo add dmc-diagnostic
```

## Quick start

```rust
use dmc_diagnostic::Code;
use duck_diagnostic::DiagnosticEngine;

let mut engine: DiagnosticEngine<Code> = DiagnosticEngine::new();
```

## Docs

- [crates.io](https://crates.io/crates/dmc-diagnostic)
- [docs.rs](https://docs.rs/dmc-diagnostic)
- Full docs in the [dmc-docs/](../dmc-docs/dmc-diagnostic/) folder (per-source-file walkthrough)

## Contributing

See [`../CONTRIBUTING.md`](../CONTRIBUTING.md).

## License

MIT. See [`../LICENSE`](../LICENSE).
