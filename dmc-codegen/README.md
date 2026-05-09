<p align="center">
  <img src="../public/logo-dark.svg" alt="dmc-codegen" width="120"/>
</p>

<h1 align="center">dmc-codegen</h1>

<p align="center">
  HTML and MDX body emitters for the dmc compiler.
</p>

<p align="center">
  <a href="../LICENSE">MIT</a> -
  <a href="../CHANGELOG.md">Changelog</a> -
  <a href="../CONTRIBUTING.md">Contributing</a> -
  <a href="https://crates.io/crates/dmc-codegen">crates.io</a> -
  <a href="https://docs.rs/dmc-codegen">docs.rs</a>
</p>

<p align="center">
  <a href="https://crates.io/crates/dmc-codegen"><img src="https://img.shields.io/crates/v/dmc-codegen.svg" alt="crates.io"/></a>
  <a href="https://docs.rs/dmc-codegen"><img src="https://docs.rs/dmc-codegen/badge.svg" alt="docs.rs"/></a>
  <a href="../LICENSE"><img src="https://img.shields.io/crates/l/dmc-codegen.svg" alt="MIT"/></a>
</p>

---

## Install

```sh
cargo add dmc-codegen
```

## Quick start

```rust
use dmc_codegen::{HtmlEmitter, Walker};

let mut emitter = HtmlEmitter::new();
Walker::new().walk(document, &mut emitter);
let html = emitter.into_string();
```

## Docs

- [crates.io](https://crates.io/crates/dmc-codegen)
- [docs.rs](https://docs.rs/dmc-codegen)
- Full docs in the [dmc-docs/](../dmc-docs/dmc-codegen/) folder (per-source-file walkthrough)

## Contributing

See [`../CONTRIBUTING.md`](../CONTRIBUTING.md).

## License

MIT. See [`../LICENSE`](../LICENSE).
