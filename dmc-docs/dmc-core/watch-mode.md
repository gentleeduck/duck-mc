# Watch mode

How `dmc build --watch` (and `dmc dev`) runs incrementally.

## Implementation

`dmc-core::engine::watch::watch_run`. Wraps `Engine::run` in a
notify + debouncer-mini loop.

```rust
pub fn watch_run(cfg: EngineConfig) -> std::io::Result<()> {
    let mut debouncer = new_debouncer(Duration::from_millis(200), |events| {
        // ...
    })?;

    for collection in &cfg.collections {
        let pattern_root = derive_root(&collection.pattern);
        debouncer.watcher().watch(&pattern_root, RecursiveMode::Recursive)?;
    }

    // initial build
    Engine::new(cfg.clone()).run()?;

    // event loop
    loop {
        let events = rx.recv()?;
        let touched = events
            .into_iter()
            .filter(|e| matches_any_pattern(&e.path, &cfg.collections))
            .collect::<Vec<_>>();

        if touched.is_empty() { continue; }

        let _ = Engine::new(cfg.clone()).run();   // ignore errors; keep watching
    }
}
```

## Debounce

200 ms. Multiple writes within that window collapse into one
rebuild. Editors that save twice (autosave + format-on-save) don't
trigger two builds.

## What gets watched

For each collection's `pattern`, the longest non-glob prefix is the
watched root.

| pattern | watched root |
|---------|-------------|
| `content/**/*.mdx` | `content` |
| `posts/*.md` | `posts` |
| `**/*.{md,mdx}` | repo root |

Recursive watch picks up new files matching the pattern.

## Cache during watch

The persistent file cache survives across rebuilds. Most files
unchanged -> cache hit -> sub-millisecond per file.

The math cache is in-memory (loaded once at startup, saved at end).
Watch mode rewrites it on each rebuild's clean exit; if you Ctrl-C
mid-build, the math cache may lose unsaved entries. Next rebuild
re-renders those.

## Hot-reload integration

Frameworks that watch the output directory pick up `.dmc/*.json`
changes:

- **Next.js**: imports `.dmc/Post.json`. Webpack watches the JSON
  file; HMR re-renders pages.
- **Vite**: same, but for any Vite-based framework (Astro, Remix,
  SolidStart, SvelteKit).
- **Astro dev server**: same; uses Vite.

dmc itself does not push HMR signals. It writes the JSON; the
framework's watcher does the rest.

## Process model

Watch mode keeps the dmc CLI running. The framework dev server runs
separately. Two processes, communicating via the filesystem.

```mermaid
flowchart LR
    Editor[editor save] -->|fs event| DMC[dmc watch]
    DMC -->|writes| Out[.dmc/*.json]
    Out -->|fs event| FW[framework dev server]
    FW -->|HMR| Browser
```

## Failure modes

| failure | behaviour |
|---------|-----------|
| transient editor lock | next event triggers rebuild; ok |
| invalid syntax in source | diagnostic emitted; build continues for other files |
| schema validation fails | record falls back to raw; diag printed |
| file deleted | record dropped from output JSON |
| pattern matches nothing | empty array emitted; no error |

The watcher never exits on error (except SIGINT). Crash recovery is
"save again" -> next event triggers rebuild.

## Limits

- One watcher per `Engine::run` instance. Re-running with a
  different config requires restarting the watch.
- Cross-filesystem mounts (NFS, SSHFS) may not deliver events
  reliably. Use polling fallback:

```bash
dmc dev --poll-ms 1000
```

## Polling fallback

```rust
let watcher = if cfg.poll_ms > 0 {
    PollWatcher::with_config(...)
} else {
    RecommendedWatcher::new(...)
};
```

Polling re-stat()s every file at the interval. Heavier, but works
on any filesystem.

## Triggering a manual rebuild

```bash
touch dmc.config.ts
```

Touching the config file is matched by the recursive watch on the
config root and triggers a full rebuild (config change invalidates
all caches via the fingerprint).

For a clean rebuild without restarting watch:

```bash
dmc build --no-cache --once
```

(separate run, doesn't disturb the watch process).
