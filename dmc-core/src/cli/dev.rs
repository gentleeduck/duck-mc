use std::path::PathBuf;

use crate::cli::config::ConfigFile;

#[derive(clap::Args)]
pub(crate) struct DevCmd {
  #[arg(long, default_value = "dmc.toml")]
  pub config: PathBuf,
}

impl DevCmd {
  /// `dmc dev`: initial build, then watch every collection's `base_dir`
  /// via notify; rebuild on .md / .mdx / .yaml / .json change.
  pub(crate) fn run(self) -> std::io::Result<()> {
    // use notify_debouncer_mini::{new_debouncer, notify::RecursiveMode};
    // use std::sync::mpsc::channel;
    // use std::time::{Duration, Instant};

    let cfg = ConfigFile::load_engine_cfg(&self.config)?;
    // print_build_result(dmc::run(&cfg)?);

    // let (tx, rx) = channel();
    // let mut deb = new_debouncer(Duration::from_millis(50), tx)
    //   .map_err(|e| std::io::Error::other(e.to_string()))?;
    // let mut roots: Vec<PathBuf> = cfg.collections.iter().map(|c| c.base_dir.clone()).collect();
    // roots.push(config.clone());
    // for r in &roots {
    //   if r.exists() {
    //     deb
    //       .watcher()
    //       .watch(r, RecursiveMode::Recursive)
    //       .map_err(|e| std::io::Error::other(e.to_string()))?;
    //   }
    // }
    // println!("👀 watching {} root(s) — Ctrl-C to stop", roots.len());
    // while let Ok(events) = rx.recv() {
    //   let Ok(events) = events else { continue };
    //   let start = Instant::now();
    //   let cfg = match ConfigFile::load_engine_cfg(&config) {
    //     Ok(c) => c,
    //     Err(e) => {
    //       eprintln!("config reload failed: {e}");
    //       continue;
    //     },
    //   };
    //   // Determine which collections were affected. If config file changed, rebuild all.
    //   let touched_paths: Vec<_> = events.iter().map(|e| e.path.clone()).collect();
    //   let config_changed =
    //     touched_paths.iter().any(|p| p.canonicalize().ok() == config.canonicalize().ok());
    //   let scoped: Vec<_> = if config_changed {
    //     cfg.collections.to_vec()
    //   } else {
    //     cfg
    //       .collections
    //       .iter()
    //       .filter(|c| {
    //         let base = c.base_dir.canonicalize().unwrap_or_else(|_| c.base_dir.clone());
    //         touched_paths.iter().any(|p| {
    //           let p_abs = p.canonicalize().unwrap_or_else(|_| p.clone());
    //           p_abs.starts_with(&base)
    //         })
    //       })
    //       .cloned()
    //       .collect()
    //   };
    //   if scoped.is_empty() {
    //     continue;
    //   }
    //   let scoped_cfg = dmc::EngineConfig { collections: scoped, ..cfg };
    //   match dmc::run(&scoped_cfg) {
    //     Ok(rep) => {
    //       println!("\n↻ rebuilt {} collection(s) in {:?}", rep.collections.len(), start.elapsed());
    //       print_build_result(rep);
    //     },
    //     Err(e) => eprintln!("build error: {e}"),
    //   }
    // }
    Ok(())
  }
}

///// Pretty-print a build's outcome to stdout: per-collection record counts
///// + any non-fatal validation errors.
// fn print_build_result(report: dmc::EngineReport) {
//   for c in &report.collections {
//     println!("✓ {} — {} records → {}", c.name, c.records, c.output_path.display());
//   }
//   if !report.errors.is_empty() {
//     for e in &report.errors {
//       eprintln!("  \x1b[31m✗\x1b[0m {}: \x1b[2m{}\x1b[0m", e.file.display(), e.message);
//     }
//   }
// }
