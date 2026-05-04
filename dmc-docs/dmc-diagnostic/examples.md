# dmc-diagnostic examples

## Emit a diagnostic from a transformer

```rust
use dmc_diagnostic::Code;
use duck_diagnostic::{Diagnostic, DiagnosticEngine, Label};

fn report_missing_file(engine: &mut DiagnosticEngine<Code>, span: duck_diagnostic::Span) {
    engine.emit(
        Diagnostic::new(Code::ImportFileNotFound, "code-import: file not on disk")
            .with_label(Label::primary(span, Some("requested here".into())))
            .with_help("check the `file=` attr"),
    );
}
```

`with_label` and `with_help` are builder methods on `Diagnostic`.

## Read codes off a finished engine

```rust
use dmc_diagnostic::Code;
use duck_diagnostic::DiagnosticEngine;

let engine: DiagnosticEngine<Code> = DiagnosticEngine::new();
// ... compile fills it ...

for d in engine.iter() {
    println!("[{}] {}", d.code.code(), d.message);
}
```

`Code::code()` returns the canonical id ("E001", "T009"). `Code::severity()`
returns `Severity::Error` or `Severity::Warning`.

## Custom code from a third-party transformer

```rust
use dmc_diagnostic::Code;
use duck_diagnostic::{Diagnostic, Severity};

let _ = Diagnostic::new(
    Code::Custom { code: "X100".into(), severity: Severity::Warning },
    "my plugin: unrecognised attr",
);
```

Use only when forking the upstream `Code` enum is not viable.
