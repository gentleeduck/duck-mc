---
"@gentleduck/md": patch
---

Embed grammar + theme assets directly into the napi `.node` binary using
`include_dir!`. Previously `dmc-highlight` loaded grammars and themes via
`SyntaxSet::load_from_folder(env!("CARGO_MANIFEST_DIR")/...)`, which baked
the build-time absolute path into the compiled binary. On any machine
that wasn't the CI runner the path didn't exist, syntect panicked with
`load grammars-sublime: WalkDir(...) NotFound`, and `native.build`
returned a partial report — making `report.collections is not iterable`
appear in callers like `apps/duck`.
