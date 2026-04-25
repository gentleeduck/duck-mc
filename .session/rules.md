# Session Rules — autonomous build of MDX compiler

## Goal
Drop-in Rust replacement for the role Velite plays in `apps/duck`. See `SURVEY.md` for parity contract.

## Per-tick procedure
1. Check elapsed: read `.session/start`. If now - start >= 16200s → write `FINAL.md`, stop.
2. Read `PROGRESS.md`. Pick first unchecked atomic task.
3. Dispatch ONE Agent (general-purpose) with self-contained prompt:
   - exact files
   - exact behavior
   - test that covers the new behavior MUST be written or extended in same task
   - run `cargo build`, `cargo test`, `cargo clippy --all-targets -- -D warnings`
   - do NOT commit
4. Verify return: read changed files, run cargo myself.
5. Pass → tick `[x]`, append `.session/log.md`, git commit `<area>: <one-line> (<id>)`.
6. Fail → revert via `git checkout -- <paths>`, mark `[⚠]` with reason in log, pick next task.
7. 3 consecutive failures → write `HALT.md` and stop.

## Tests are mandatory
- Every behavior change MUST add or update a test.
- Test files live in `<crate>/tests/` (integration) or under `#[cfg(test)] mod tests` for unit tests near the code.
- Use `pretty_assertions` for diff-able failures. Use `insta` for snapshots when added.
- A task without a test counts as `[⚠]` and gets reopened.

## Style
- No new top-level deps without note in `.session/log.md` why.
- No refactor outside the task.
- No new `*.md` docs unless the task asks.
- Commit subjects ≤50 chars, conventional-commits style.
- Never `git push`.

## Phase 16 (continuous expansion)
When all phases 1-15 ticked, do NOT stop until deadline. Append new atomic tasks under Phase 16 (footnotes, math, callouts, LSP, WASM, more parity fixtures, fuzz corpus growth, etc.) and continue dispatching.

## Stop conditions
- Deadline elapsed → `FINAL.md`
- 3 consecutive failures → `HALT.md`
- All Phase 16 ideas exhausted (unlikely) → `IDLE.md` and stop
