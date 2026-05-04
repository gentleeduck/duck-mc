use std::io::{BufRead, BufReader, Write};
use std::sync::atomic::Ordering;
use std::{
  process::{Child, ChildStdin, ChildStdout, Command, Stdio},
  sync::{
    Mutex, OnceLock,
    atomic::{AtomicU64, AtomicUsize},
  },
};

use serde_json::{Value, json};

use crate::engine::config::EngineConfig;

pub struct Sidecar {
  stdin: ChildStdin,
  stdout: BufReader<ChildStdout>,
  _child: Child,
}

static POOL: OnceLock<Vec<Mutex<Option<Sidecar>>>> = OnceLock::new();
static REQ_ID: AtomicU64 = AtomicU64::new(0);
static NEXT_SLOT: AtomicUsize = AtomicUsize::new(0);

fn pool_size() -> usize {
  std::env::var("DMC_SIDECAR_POOL_SIZE")
    .ok()
    .and_then(|s| s.parse().ok())
    .unwrap_or_else(|| std::thread::available_parallelism().map(|p| p.get().min(4)).unwrap_or(2))
}

fn pool() -> &'static Vec<Mutex<Option<Sidecar>>> {
  POOL.get_or_init(|| (0..pool_size()).map(|_| Mutex::new(None)).collect())
}

impl Sidecar {
  pub fn new() -> Option<Self> {
    let entry = std::env::var("dmc_SIDECAR").ok().unwrap_or_else(|| "dmc-sidecar/index.mjs".into());
    let mut child = Command::new("node")
      .arg(&entry)
      .stdin(Stdio::piped())
      .stdout(Stdio::piped())
      .stderr(Stdio::null())
      .spawn()
      .ok()?;
    let stdin = child.stdin.take().unwrap();
    let stdout = BufReader::new(child.stdout.take().unwrap());
    Some(Self { stdin, stdout, _child: child })
  }
}

pub fn run_sidecar(markdown: &str, cfg: &EngineConfig) -> Option<String> {
  let pool = pool();
  let n = pool.len();

  // try every slot via try_lock first - grab whichever idle
  let mut guard = None;
  for _ in 0..n {
    let idx = NEXT_SLOT.fetch_add(1, Ordering::Relaxed) % n;
    if let Ok(g) = pool[idx].try_lock() {
      guard = Some(g);
      break;
    }
  }
  // all busy -> block on round-robin pick
  let mut guard = match guard {
    Some(g) => g,
    None => {
      let idx = NEXT_SLOT.fetch_add(1, Ordering::Relaxed) % n;
      pool[idx].lock().ok()?
    },
  };

  if guard.is_none() {
    *guard = Some(Sidecar::new()?);
  }
  let child = guard.as_mut().unwrap();

  let id = REQ_ID.fetch_add(1, Ordering::Relaxed);
  let merge = |a: &Vec<Value>, b: &Vec<Value>| -> Value { Value::Array(a.iter().chain(b.iter()).cloned().collect()) };
  let remark_md = cfg.compile.effective_markdown_remark_plugins();
  let remark_mdx = cfg.compile.effective_mdx_remark_plugins();
  let rehype_md = cfg.compile.effective_markdown_rehype_plugins();
  let rehype_mdx = cfg.compile.effective_mdx_rehype_plugins();
  let req = json!({
    "id": id,
    "markdown": markdown,
    "remarkPlugins": merge(&remark_md, &remark_mdx),
    "rehypePlugins": merge(&rehype_md, &rehype_mdx),
  });

  writeln!(child.stdin, "{}", req).ok()?;
  child.stdin.flush().ok()?;
  let mut line = String::new();
  child.stdout.read_line(&mut line).ok()?;
  let parsed: Value = serde_json::from_str(&line).ok()?;
  parsed.get("html").and_then(|v| v.as_str()).map(String::from)
}

/// Drop every live child in the pool; subsequent `run_sidecar` calls
/// re-spawn fresh children. Bench-only; not part of the normal build flow.
pub fn shutdown_pool() {
  if let Some(pool) = POOL.get() {
    for slot in pool {
      if let Ok(mut g) = slot.lock() {
        // Dropping the Sidecar drops its ChildStdin -> the node child sees
        // stdin close, hits readline 'close', exits 0.
        *g = None;
      }
    }
  }
}
