use std::io::{BufRead, BufReader, Write};
use std::{
  process::{Child, ChildStdin, ChildStdout, Command, Stdio},
  sync::{Mutex, OnceLock, atomic::AtomicU64},
};

use serde_json::{Value, json};

use crate::engine::config::EngineConfig;

pub struct Sidecar {
  stdin: ChildStdin,
  stdout: BufReader<ChildStdout>,
  _child: Child,
}

static SIDECAR: OnceLock<Mutex<Option<Sidecar>>> = OnceLock::new();
static REQ_ID: AtomicU64 = AtomicU64::new(0);

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

    Some(Sidecar { stdin, stdout, _child: child })
  }
}

pub fn run_sidecar(markdown: &str, cfg: &EngineConfig) -> Option<String> {
  let mut guard = SIDECAR.get_or_init(|| Mutex::new(None)).lock().ok()?;
  if guard.is_none() {
    *guard = Some(Sidecar::new()?);
  }

  let child = guard.as_mut().unwrap();
  let id = REQ_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
  let merge = |a: &Vec<Value>, b: &Vec<Value>| -> Value {
    Value::Array(a.iter().chain(b.iter()).cloned().collect())
  };

  let req = json!({
    "id": id,
    "markdown": markdown,
    "remarkPlugins": merge(&cfg.markdown_remark_plugins, &cfg.mdx_remark_plugins),
    "rehypePlugins": merge(&cfg.markdown_rehype_plugins, &cfg.mdx_rehype_plugins),
  });

  writeln!(child.stdin, "{}", req).ok()?;
  child.stdin.flush().ok()?;

  let mut line = String::new();
  child.stdout.read_line(&mut line).ok()?;
  let parsed: Value = serde_json::from_str(&line).ok()?;
  parsed.get("html").and_then(|v| v.as_str()).map(String::from)
}
