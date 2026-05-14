//! Persistent per-file compile cache at `<output_dir>/.cache/dmc/`,
//! one `{16-hex blake3}.json` per record. Cache hits are O(read + parse).
//! Key encodes dmc version + source bytes + path + config fingerprint;
//! nothing overwrites in place.

use blake3::Hasher;
use serde_json::Value;
use std::path::{Path, PathBuf};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone)]
pub struct FileCache {
  dir: PathBuf,
}

impl FileCache {
  /// Returns `None` (cache disabled) if the dir can't be created.
  pub fn open(dir: PathBuf) -> Option<Self> {
    std::fs::create_dir_all(&dir).ok()?;
    Some(Self { dir })
  }

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

  pub fn get(&self, key: &str) -> Option<Value> {
    let p = self.path_for(key);
    let s = std::fs::read_to_string(p).ok()?;
    serde_json::from_str(&s).ok()
  }

  /// Write errors are swallowed: a cache failure must not break the build.
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

/// Empty vec on serialisation failure (cache still works, collides
/// across non-serialisable configs).
pub fn fingerprint<T: serde::Serialize>(cfg: &T) -> Vec<u8> {
  let Ok(json) = serde_json::to_vec(cfg) else { return Vec::new() };
  blake3::hash(&json).as_bytes().to_vec()
}
