//! The process-wide mermaid renderer: one node child holding one headless
//! browser, fed every diagram over NDJSON by `renderer.mjs`. Running `mmdc`
//! per diagram launches node and Chrome for each diagram and theme, which
//! was nearly all of a cold docs build: 560 launches, 94% of its CPU.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, LazyLock, Mutex};

use serde::Deserialize;
use serde_json::{Value, json};

const SCRIPT: &str = include_str!("renderer.mjs");

/// One diagram to draw, in the terms `mmdc` takes it.
pub(super) struct Job<'a> {
  pub source: &'a str,
  pub theme: &'a str,
  pub background_color: &'a str,
  /// The mermaid `initialize` config `mmdc --configFile` would read.
  pub config: &'a Value,
}

/// The SVG, or mermaid's reason for rejecting the diagram.
type Reply = Result<String, String>;

/// Render `job` on the shared renderer. `None` when no renderer can run
/// here (no node, no `@mermaid-js/mermaid-cli` package behind `mmdc`, a
/// browser that won't launch); the caller runs `mmdc` for the diagram.
pub(super) fn render(job: &Job, puppeteer_config: Option<&Path>) -> Option<Reply> {
  // A renderer that exits mid-request (it crashed, or was closing for
  // idleness as the request arrived) never answers; retry on a fresh one.
  for _ in 0..2 {
    if let Some(reply) = renderer(puppeteer_config)?.send(job) {
      return Some(reply);
    }
  }
  None
}

enum Slot {
  Up(Arc<Renderer>),
  /// It could not start here: every diagram goes to `mmdc` instead.
  Unavailable,
}

/// One renderer per puppeteer config file, the one input fixed at launch.
static SLOTS: LazyLock<Mutex<HashMap<Option<PathBuf>, Slot>>> = LazyLock::new(Default::default);

fn renderer(puppeteer_config: Option<&Path>) -> Option<Arc<Renderer>> {
  let key = puppeteer_config.map(Path::to_path_buf);
  let mut slots = SLOTS.lock().unwrap();
  match slots.get(&key) {
    Some(Slot::Up(r)) if r.is_alive() => return Some(r.clone()),
    Some(Slot::Unavailable) => return None,
    _ => {},
  }
  // Spawned under the lock: every other thread is after this same renderer.
  let up = Renderer::spawn(puppeteer_config).map(Arc::new);
  slots.insert(key, up.clone().map_or(Slot::Unavailable, Slot::Up));
  up
}

struct Renderer {
  stdin: Mutex<ChildStdin>,
  waiting: Arc<Mutex<Waiting>>,
  next_id: AtomicU64,
}

/// Requests sent and not yet answered. The reader marks the process dead
/// and drops these under the same lock [`Renderer::send`] registers under,
/// so no request can be registered after the last reply was read.
struct Waiting {
  alive: bool,
  replies: HashMap<u64, Sender<Reply>>,
}

#[derive(Deserialize)]
struct Answer {
  id: u64,
  svg: Option<String>,
  error: Option<String>,
}

impl Renderer {
  fn spawn(puppeteer_config: Option<&Path>) -> Option<Self> {
    let cli = mermaid_cli_dir()?;
    let mut cmd = Command::new("node");
    cmd.args(["--input-type=module", "-e", SCRIPT]).arg(&cli);
    if let Some(p) = puppeteer_config {
      cmd.arg(p);
    }
    let mut child = cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().ok()?;
    let (stdin, stdout) = (child.stdin.take()?, child.stdout.take()?);
    let mut stdout = BufReader::new(stdout);
    let mut line = String::new();
    let ready =
      stdout.read_line(&mut line).is_ok() && serde_json::from_str::<Value>(&line).is_ok_and(|v| v["ready"] == true);
    if !ready {
      let _ = child.kill();
      let _ = child.wait();
      return None;
    }
    let waiting = Arc::new(Mutex::new(Waiting { alive: true, replies: HashMap::new() }));
    let shared = waiting.clone();
    std::thread::spawn(move || read_replies(stdout, child, &shared));
    Some(Self { stdin: Mutex::new(stdin), waiting, next_id: AtomicU64::new(0) })
  }

  fn is_alive(&self) -> bool {
    self.waiting.lock().unwrap().alive
  }

  /// `None` when the process exited before answering.
  fn send(&self, job: &Job) -> Option<Reply> {
    let id = self.next_id.fetch_add(1, Ordering::Relaxed);
    let (tx, rx) = channel();
    {
      let mut w = self.waiting.lock().unwrap();
      if !w.alive {
        return None;
      }
      w.replies.insert(id, tx);
    }
    let mut line = json!({
      "id": id,
      "source": job.source,
      "theme": job.theme,
      "backgroundColor": job.background_color,
      "config": job.config,
    })
    .to_string();
    line.push('\n');
    if self.stdin.lock().unwrap().write_all(line.as_bytes()).is_err() {
      let mut w = self.waiting.lock().unwrap();
      w.alive = false;
      w.replies.remove(&id);
      return None;
    }
    rx.recv().ok()
  }
}

/// Hand each reply to the request waiting on it. Once the process exits,
/// drop every request still waiting, which fails its `recv`.
fn read_replies(stdout: BufReader<ChildStdout>, mut child: Child, waiting: &Mutex<Waiting>) {
  for line in stdout.lines() {
    let Ok(line) = line else { break };
    let Ok(a) = serde_json::from_str::<Answer>(&line) else { continue };
    let reply = a.svg.ok_or_else(|| a.error.unwrap_or_else(|| "renderer sent no svg".into()));
    if let Some(tx) = waiting.lock().unwrap().replies.remove(&a.id) {
      let _ = tx.send(reply);
    }
  }
  {
    let mut w = waiting.lock().unwrap();
    w.alive = false;
    w.replies.clear();
  }
  let _ = child.wait();
}

/// The `@mermaid-js/mermaid-cli` package behind the `mmdc` on `PATH`, so
/// the renderer runs the very code `mmdc` would.
fn mermaid_cli_dir() -> Option<PathBuf> {
  let path = std::env::var_os("PATH")?;
  let bin = std::env::split_paths(&path).map(|d| d.join("mmdc")).find(|p| p.is_file())?;
  package_behind(&bin)
}

/// A symlinked bin (npm, bun, Homebrew) resolves into `<package>/src/`; a
/// shim script (pnpm, yarn) sits in `node_modules/.bin`, beside the package.
fn package_behind(bin: &Path) -> Option<PathBuf> {
  let linked = std::fs::canonicalize(bin)
    .ok()?
    .ancestors()
    .skip(1)
    .find(|d| d.join("package.json").is_file())
    .map(Path::to_path_buf);
  let beside = bin.parent().map(|d| d.join("../@mermaid-js/mermaid-cli"));
  [linked, beside].into_iter().flatten().filter_map(|d| std::fs::canonicalize(d).ok()).find(|d| is_mermaid_cli(d))
}

fn is_mermaid_cli(dir: &Path) -> bool {
  std::fs::read_to_string(dir.join("package.json"))
    .ok()
    .and_then(|s| serde_json::from_str::<Value>(&s).ok())
    .is_some_and(|v| v["name"] == "@mermaid-js/mermaid-cli")
}

#[cfg(test)]
mod tests {
  use super::super::Mermaid;
  use super::*;
  use std::fs;

  fn write_package(dir: &Path, name: &str) {
    fs::create_dir_all(dir).unwrap();
    fs::write(dir.join("package.json"), format!(r#"{{"name":"{name}"}}"#)).unwrap();
  }

  #[cfg(unix)]
  #[test]
  fn finds_the_package_a_symlinked_bin_points_into() {
    let root = tempfile::tempdir().unwrap();
    let pkg = root.path().join("lib/node_modules/@mermaid-js/mermaid-cli");
    write_package(&pkg, "@mermaid-js/mermaid-cli");
    fs::create_dir_all(pkg.join("src")).unwrap();
    fs::write(pkg.join("src/cli.js"), "").unwrap();
    fs::create_dir_all(root.path().join("bin")).unwrap();
    std::os::unix::fs::symlink("../lib/node_modules/@mermaid-js/mermaid-cli/src/cli.js", root.path().join("bin/mmdc"))
      .unwrap();
    assert_eq!(package_behind(&root.path().join("bin/mmdc")), Some(fs::canonicalize(&pkg).unwrap()));
  }

  #[test]
  fn finds_the_package_beside_a_node_modules_shim() {
    let root = tempfile::tempdir().unwrap();
    let modules = root.path().join("node_modules");
    let pkg = modules.join("@mermaid-js/mermaid-cli");
    write_package(&pkg, "@mermaid-js/mermaid-cli");
    fs::create_dir_all(modules.join(".bin")).unwrap();
    fs::write(modules.join(".bin/mmdc"), "#!/bin/sh\n").unwrap();
    assert_eq!(package_behind(&modules.join(".bin/mmdc")), Some(fs::canonicalize(&pkg).unwrap()));
  }

  #[test]
  fn finds_nothing_behind_a_wrapper_script() {
    // e.g. an `mmdc` that runs the mermaid-cli image: no package to load,
    // so every diagram keeps going through `mmdc`
    let root = tempfile::tempdir().unwrap();
    write_package(root.path(), "some-project");
    fs::create_dir_all(root.path().join("bin")).unwrap();
    fs::write(root.path().join("bin/mmdc"), "#!/bin/sh\n").unwrap();
    assert_eq!(package_behind(&root.path().join("bin/mmdc")), None);
  }

  // The tests below need mermaid-cli and its browser installed, and pass
  // vacuously where they aren't (CI): there is nothing there to compare.

  /// The renderer may change only the launches: the SVG has to be the one
  /// `mmdc` writes, byte for byte, or every cached diagram goes stale.
  #[test]
  fn draws_the_svg_mmdc_draws() {
    if !Mermaid::mmdc_available() {
      return;
    }
    let m = Mermaid::default();
    let config = m.build_mermaid_config();
    let source = "flowchart LR\n  A[Start] --> B{Ok?}\n  B -->|yes| C[Done]\n  B -->|no| A";
    for theme in ["default", "dark"] {
      let job = Job { source, theme, background_color: "transparent", config: &config };
      let Some(shared) = render(&job, None) else { return };
      assert_eq!(shared.map(|svg| m.post_process(&svg)), m.render_mmdc(source, theme), "theme {theme}");
    }
  }

  #[test]
  fn answers_each_request_with_its_own_diagram() {
    if !Mermaid::mmdc_available() {
      return;
    }
    let config = Mermaid::default().build_mermaid_config();
    let replies: Vec<_> = std::thread::scope(|s| {
      let handles: Vec<_> = (0..6)
        .map(|i| {
          let config = &config;
          s.spawn(move || {
            let source = format!("flowchart LR\n  A[only-in-{i}] --> B");
            render(&Job { source: &source, theme: "default", background_color: "transparent", config }, None)
          })
        })
        .collect();
      handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    for (i, reply) in replies.into_iter().enumerate() {
      let Some(reply) = reply else { return };
      let svg = reply.unwrap();
      assert!(svg.contains(&format!("only-in-{i}")), "request {i} got another diagram");
    }
  }

  #[test]
  fn reports_a_broken_diagram_as_a_render_error() {
    if !Mermaid::mmdc_available() {
      return;
    }
    let config = Mermaid::default().build_mermaid_config();
    let job =
      Job { source: "flowchart LR\n  A -->", theme: "default", background_color: "transparent", config: &config };
    // `Some(Err)`, not `None`: mermaid's verdict, not a dead renderer
    // that would send the diagram on to `mmdc` for a second failure
    if let Some(reply) = render(&job, None) {
      assert!(reply.is_err(), "a parse error rendered: {reply:?}");
    }
  }
}
