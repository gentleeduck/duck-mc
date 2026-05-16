---
"@gentleduck/md": patch
---

`duck-md dev` (alias `duck-md watch`) now skips rebuilds on saves that don't change file content. After the initial build, the CLI seeds a `Map<absPath, sha256>` of every `.md` / `.mdx` file under `root`; chokidar change events re-hash and short-circuit when the hash matches the stored one (`[duck-md] no-op (<rel> unchanged)` log line). The same dedupe applies to the config file. `add` / `unlink` events still trigger a full rebuild. This sits above the existing per-file blake3 cache in `dmc-core`, so real edits remain incremental and bare saves cost nothing.
