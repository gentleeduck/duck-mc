/// Identity of the bytes being lexed. Threaded through every emitted `Span`
/// (via `path`). Cheap to clone: `path` is `Arc<str>`.
pub struct SourceMeta {
  /// Display string used as `Span.file`. Refcounted so every span shares one
  /// allocation. Canonical filesystem path for `File`, `"<stdin>"` for
  /// `Stdin`, fixture name for `Inline`, stable identifier (e.g. LSP URI)
  /// for `Memory`.
  pub path: std::sync::Arc<str>,
  /// Drives path resolution and caching policy.
  pub origin: Origin,
}

/// Where the source bytes were obtained — determines what callers can do
/// beyond display:
///
/// - `File`: parent dir known, relative `file=...` resolves, safe to cache
///   by `(path, mtime)`.
/// - `Stdin`: one-shot, no parent dir, no cache.
/// - `Inline`: tests / REPL / doc-test snippet; `path` is synthetic.
/// - `Memory`: LSP unsaved buffer; `version` is the source of truth.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Origin {
  /// On-disk file. Holds the resolved path for sibling reads, watches, etc.
  File(std::path::PathBuf),
  /// Piped in via stdin. No parent directory, no re-read.
  Stdin,
  /// Hard-coded fixture, REPL eval, or doc-test snippet. The `&'static str`
  /// is a human-readable label (e.g. `"E004-fixture"`).
  Inline(&'static str),
  /// In-RAM buffer (LSP unsaved doc, generated content) — not on disk.
  Memory,
}
