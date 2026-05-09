use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::time::{Duration, Instant};

use dmc_diagnostic::{Code, DiagResult};
use duck_diagnostic::{Diagnostic, DiagnosticEngine, diag, print_all_smart};
use notify_debouncer_mini::{new_debouncer, notify::RecursiveMode};

use crate::{Engine, engine::config::EngineConfig};

/// Initial build, then watch every collection's `base_dir` (plus the
/// config file) and rebuild on change.
#[derive(clap::Args)]
pub struct DevCmd {
  #[arg(long, default_value = "dmc.toml")]
  pub config: PathBuf,
  #[arg(short, long)]
  pub strict: bool,
  #[arg(long)]
  pub clean: bool,
  /// Debounce window for filesystem events (ms).
  #[arg(long, default_value_t = 100)]
  pub debounce: u64,
}

impl DevCmd {
  pub fn run(self) -> DiagResult<Diagnostic<Code>> {
    let mut cfg = EngineConfig::load(&self.config)?;
    if self.strict {
      cfg.strict = true;
    }
    if self.clean {
      cfg.clean = true;
    }

    rebuild(&cfg, &self.config, "initial")?;

    let (tx, rx) = channel();
    let mut deb = new_debouncer(Duration::from_millis(self.debounce), tx).map_err(|e| {
      diag!(
        Code::Custom { code: String::from("N001"), severity: duck_diagnostic::Severity::Note },
        format!("watch error: {}", e.to_string())
      )
    })?;

    let mut roots: Vec<PathBuf> = cfg.collections.iter().map(|c| c.base_dir.clone()).collect();
    roots.push(self.config.clone());

    for r in &roots {
      if r.exists() {
        deb.watcher().watch(r, RecursiveMode::Recursive).map_err(|e| {
          diag!(
            Code::Custom { code: String::from("N001"), severity: duck_diagnostic::Severity::Note },
            format!("watch error: {}", e.to_string())
          )
        })?;
      }
    }

    println!("watching {} root(s) - Ctrl-C to stop", roots.len());

    while let Ok(events) = rx.recv() {
      let Ok(events) = events else { continue };
      let touched: Vec<PathBuf> = events.iter().map(|e| e.path.clone()).collect();
      let cfg_canon = self.config.canonicalize().ok();
      let config_changed = touched.iter().any(|p| p.canonicalize().ok() == cfg_canon);

      if config_changed {
        let mut next = EngineConfig::load(&self.config)?;

        if self.strict {
          next.strict = true;
        }
        if self.clean {
          next.clean = true;
        }

        cfg = next;
      }

      rebuild(&cfg, &self.config, if config_changed { "config" } else { "files" })?;
    }

    Ok(diag!(
      Code::Custom { code: String::from("N001"), severity: duck_diagnostic::Severity::Note },
      format!("watching {} root(s) - Ctrl-C to stop", roots.len())
    ))
  }
}

fn rebuild(cfg: &EngineConfig, config_path: &std::path::Path, _trigger: &str) -> DiagResult {
  let mut diag_engine = DiagnosticEngine::<Code>::new();
  let started = Instant::now();
  Engine::run(cfg, Some(config_path), &mut diag_engine)?;
  let elapsed = started.elapsed();
  print_all_smart(&diag_engine, None);

  diag_engine.emit(diag!(
    Code::Custom { code: String::from("N001"), severity: duck_diagnostic::Severity::Note },
    format!("built in {:<.3?}", elapsed)
  ));

  Ok(())
}
