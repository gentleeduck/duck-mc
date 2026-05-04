//! Persistent per-file compile cache. Skips lex/parse/transform/codegen
//! (and the sidecar dispatch) for unchanged inputs by hashing the source
//! together with the build config and stashing the resulting record on
//! disk. Cache hits are O(read JSON + parse).
//!
//! Default location: `<output_dir>/.cache/dmc/`. One file per record,
//! named `{16-hex blake3}.json`.
//!
//! Invalidation strategy: the key encodes the dmc version, the file
//! source bytes, the source path, and a serialised view of the relevant
//! `CompileConfig` fields. Any change to source or config produces a
//! different hash; nothing is ever overwritten in place.

use blake3::Hasher;
use serde_json::Value;
use std::path::{Path, PathBuf};

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// File-backed key/value store. One file per cached record.
#[derive(Debug, Clone)]
pub struct FileCache {
  dir: PathBuf,
}

impl FileCache {
  /// Open or create the cache at `dir`. Returns `None` (cache disabled)
  /// if the directory could not be created.
  pub fn open(dir: PathBuf) -> Option<Self> {
    std::fs::create_dir_all(&dir).ok()?;
    Some(Self { dir })
  }

  /// Compute a hex key for a cache entry. Inputs:
  /// - dmc version (changes invalidate every entry)
  /// - file source bytes
  /// - file path (so two identical-content files at different paths
  ///   don't collide)
  /// - opaque config fingerprint (caller-controlled)
  pub fn key(source: &[u8], path: &Path, cfg_fingerprint: &[u8]) -> String {
    let mut h = Hasher::new();
    h.update(b"dmc/v1");
    h.update(VERSION.as_bytes());
    h.update(b"\0src\0");
    h.update(source);
    h.update(b"\0path\0");
    h.update(path.to_string_lossy().as_bytes());
    h.update(b"\0cfg\0");
    h.update(cfg_fingerprint);
    let hex = h.finalize().to_hex();
    hex.as_str()[..16].to_string()
  }

  /// Load the record for `key`, returning `None` on miss or read error.
  pub fn get(&self, key: &str) -> Option<Value> {
    let p = self.path_for(key);
    let s = std::fs::read_to_string(p).ok()?;
    serde_json::from_str(&s).ok()
  }

  /// Write `value` under `key`. Errors are silently ignored; a cache
  /// failure must never break the build.
  pub fn put(&self, key: &str, value: &Value) {
    let p = self.path_for(key);
    if let Ok(json) = serde_json::to_string(value) {
      let _ = std::fs::write(p, json);
    }
  }

  fn path_for(&self, key: &str) -> PathBuf {
    self.dir.join(format!("{key}.json"))
  }
}

/// Hash any `Serialize`-able config snippet into an opaque fingerprint.
/// Returns the empty vec on serialisation failure (cache still works,
/// just collides across configs that fail to serialise).
pub fn fingerprint<T: serde::Serialize>(cfg: &T) -> Vec<u8> {
  let Ok(json) = serde_json::to_vec(cfg) else { return Vec::new() };
  blake3::hash(&json).as_bytes().to_vec()
}
