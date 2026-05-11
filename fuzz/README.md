# dmc fuzz targets

libFuzzer targets for the markdown/MDX pipeline. Run with nightly + `cargo-fuzz`:

```sh
cargo install cargo-fuzz
cargo +nightly fuzz run <target> fuzz/seeds          # seed from the spec corpus
cargo +nightly fuzz run <target> -- -max_total_time=60   # time-boxed
```

Targets:

| target              | exercises                                                        |
|---------------------|------------------------------------------------------------------|
| `fuzz_lex`          | `Lexer::scan_tokens` + token invariants (raw within source, span lengths, no token explosion) |
| `fuzz_parse`        | `dmc_parser::parse` (lossy UTF-8 input)                          |
| `fuzz_parse_strict` | `dmc_parser::parse_with` under spec-runner options              |
| `fuzz_roundtrip`    | `parse` -> `dmc_codegen::render_html`                            |
| `fuzz_compile`      | full `dmc::Compiler::compile` pipeline (lex -> parse -> transforms -> codegen) |

`fuzz/seeds/` holds one file per CommonMark 0.31.2 + GFM 0.29 spec example -
copy it into a target's working corpus, or pass it on the command line as
shown above. The generated working corpus (`fuzz/corpus/`), crash artifacts
(`fuzz/artifacts/`), and build output (`fuzz/target/`) are git-ignored.

A non-fuzzer regression smoke for the target bodies lives in
`dmc-parser/tests/fuzz_smoke.rs` so CI catches panics without running the
fuzzer.

Past fuzz finds (all fixed): two parser DoS inputs (exponential nested-`[`
link-label re-parse; infinite loop on a tab-indented list-item code block)
and a `bare-url` transformer slice panic on a `www.`-only run.
