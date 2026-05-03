/// Identity of the bytes being lexed. Threaded through every emitted `Span`
/// (via `path`) and consulted by callers that need to resolve relative paths
/// (`origin`) or invalidate caches (`version`). Cheap to clone: `path` is
/// `Arc<str>`, `origin` is small.
pub struct SourceMeta {
  /// Display string used as `Span.file`. Refcounted so every emitted span
  /// shares one allocation. Use the canonical filesystem path for `File`,
  /// `"<stdin>"` for `Stdin`, a fixture name for `Inline`, anything stable
  /// for `Memory` (e.g. an LSP URI).
  pub path: std::sync::Arc<str>,
  /// Monotonic edit counter. Caller bumps on every modification. Used by
  /// incremental layers / caches to detect staleness. Ignore (leave at 0)
  /// for one-shot lexing of a static file.
  pub version: u64,
  /// Where the bytes came from. Drives path resolution + caching policy.
  pub origin: Origin,
}

/// Where the source bytes were obtained. Determines what callers can do with
/// the document beyond display:
///
/// - `File`: parent dir is known, so relative `file=...` directives resolve;
///   on-disk content can be re-read; safe to cache by `(path, mtime)`.
/// - `Stdin`: one-shot, no parent dir, no cache.
/// - `Inline`: tests / REPL / hard-coded snippet; `path` is a synthetic name.
/// - `Memory`: LSP-style unsaved buffer; `version` is the source of truth,
///   not the filesystem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Origin {
  /// On-disk file. Holds the resolved path so callers can read siblings,
  /// watch for changes, etc.
  File(std::path::PathBuf),
  /// Piped in via stdin. No parent directory, no re-read.
  Stdin,
  /// Hard-coded fixture, REPL eval, or doc-test snippet. The static `&str`
  /// is the human-readable label (e.g. `"E004-fixture"`).
  Inline(&'static str),
  /// In-RAM buffer (LSP unsaved doc, generated content). The bytes don't
  /// live on disk; lean on `SourceMeta.version` for change tracking.
  Memory,
}
