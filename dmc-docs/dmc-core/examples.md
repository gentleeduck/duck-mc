# dmc-core examples

## Programmatic `Engine::run`

```rust
use std::path::PathBuf;
use dmc::Engine;
use dmc::engine::collection::Collection;
use dmc::engine::compile::CompileConfig;
use dmc::engine::config::EngineConfig;
use dmc_diagnostic::Code;
use duck_diagnostic::DiagnosticEngine;

fn build() -> std::io::Result<()> {
    let cfg = EngineConfig {
        root: PathBuf::from("content"),
        output_dir: PathBuf::from(".gentleduck"),
        output_format: Some("esm".into()),
        clean: false,
        cache_enabled: true,
        include_html: true,
        collections: vec![Collection {
            name: "doc".into(),
            pattern: "docs/**/*.mdx".into(),
            base_dir: PathBuf::from("content"),
            schema: None,
            single: false,
        }],
        compile: CompileConfig::default(),
        ..Default::default()
    };

    let mut diag = DiagnosticEngine::<Code>::new();
    Engine::run(&cfg, None, &mut diag)?;
    Ok(())
}
```

## One-shot compile

```rust
use std::path::Path;
use dmc::engine::compile::{Compiler, CompileConfig};
use dmc_diagnostic::Code;
use duck_diagnostic::DiagnosticEngine;

let mut diag = DiagnosticEngine::<Code>::new();
let out = Compiler::compile_with_pipeline(
    "# hello\n\n*world*",
    Path::new("<inline>"),
    &CompileConfig::default(),
    &mut diag,
);

println!("{}", out.html);
```

## Disable cache for one build

```rust
let cfg = EngineConfig { cache_enabled: false, ..base };
Engine::run(&cfg, None, &mut diag)?;
```

## Custom `CompileConfig` with multi-theme highlighter

```rust
use std::collections::BTreeMap;
use dmc_transform::{PrettyCodeOptions, PrettyCodeTheme};
use dmc::engine::compile::CompileConfig;

let mut themes = BTreeMap::new();
themes.insert("light".into(), "Catppuccin Latte".into());
themes.insert("dark".into(), "Catppuccin Mocha".into());

let cfg = CompileConfig {
    pretty_code: Some(PrettyCodeOptions {
        theme: PrettyCodeTheme::Multi(themes),
        default_mode: Some("dark".into()),
    }),
    ..CompileConfig::default()
};
```

## Force MathML engine

```rust
use dmc_transform::MathEngine;
use dmc::engine::compile::CompileConfig;

let cfg = CompileConfig {
    math_engine: Some(MathEngine::Mathml),
    ..CompileConfig::default()
};
```

Trades visual KaTeX parity for ~300 ms saving on a 1000-file kitchen
sink build.

## Read diagnostics after build

```rust
let mut diag = DiagnosticEngine::<Code>::new();
Engine::run(&cfg, None, &mut diag)?;

for d in diag.iter() {
    println!("[{}] {}", d.code.code(), d.message);
}
```

`Code::code()` is the canonical id ("E001", "T009", ...). Use to
filter, group, or fail-on-error in CI.
