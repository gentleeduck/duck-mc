---
"@gentleduck/md": patch
---

`CompileOutput` now describes the two fields it was already returning: `diagnostics` and `components`.

`compile` started reporting diagnostics in 0.6.1, but the hand-written TypeScript for its return type never mentioned them, so reading them was a type error on a value that was right there. `components` had been missing the same way for longer.

`tsc` cannot catch this — an interface that omits a field typechecks fine — so a test now pins `CompileOutput`'s serialized key set. Adding a Rust field without describing it on the TypeScript side fails that test.
