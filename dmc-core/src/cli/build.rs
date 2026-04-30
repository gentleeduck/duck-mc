use std::path::PathBuf;

use dmc::Engine;

use crate::cli::config::ConfigFile;

/// `dmc build`: load config, run the engine once, print the report.
#[derive(clap::Args)]
pub(crate) struct BuildCmd {
  #[arg(long, default_value = "dmc.toml")]
  pub config: PathBuf,
  #[arg(short, long)]
  pub strict: bool,
  #[arg(long)]
  pub clean: bool,
}

impl BuildCmd {
  /// `dmc build`: load config, run the engine once, print the report.
  /// `strict` aborts on the first validation failure, `clean` wipes
  /// `output_dir` first.
  pub(crate) fn run(self) -> std::io::Result<()> {
    let mut engine_cfg = ConfigFile::load_engine_cfg(&self.config)?;
    if self.strict {
      engine_cfg.strict = true;
    }
    if self.clean {
      engine_cfg.clean = true;
    }

    Engine::run(&engine_cfg)?;

    // // TODO: refactor the code below
    // for c in &report.collections {
    //   println!("✓ {} — {} records → {}", c.name, c.records, c.output_path.display());
    // }
    // if !report.errors.is_empty() {
    //   eprintln!();
    //   for e in &report.errors {
    //     eprintln!("  \x1b[31m✗\x1b[0m {}", e.file.display());
    //     eprintln!("    \x1b[2m{}\x1b[0m", e.message);
    //   }
    //   eprintln!("\n  \x1b[31m{}\x1b[0m validation error(s)", report.errors.len());
    // }
    Ok(())
  }
}
