# @gentleduck/md

## 0.2.1

### Patch Changes

- 721da9e: Embed grammar + theme assets directly into the napi `.node` binary using
  `include_dir!`. Previously `dmc-highlight` loaded grammars and themes via
  `SyntaxSet::load_from_folder(env!("CARGO_MANIFEST_DIR")/...)`, which baked
  the build-time absolute path into the compiled binary. On any machine
  that wasn't the CI runner the path didn't exist, syntect panicked with
  `load grammars-sublime: WalkDir(...) NotFound`, and `native.build`
  returned a partial report — making `report.collections is not iterable`
  appear in callers like `apps/duck`.

## 0.2.0

### Minor Changes

- 37bd35c: Initial npm release wired through changesets + napi-prebuilds. Bumps the
  package to 0.2.0 to track the underlying Rust crates and ships prebuilt
  `.node` binaries for 13 napi-rs canonical targets (macOS x64+arm64,
  Windows x64/x86/arm64, Linux gnu+musl on x64/arm64/armv7, Android
  arm64/armv7, FreeBSD x64).
