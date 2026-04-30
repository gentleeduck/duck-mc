use std::path::PathBuf;

#[derive(clap::Args)]
pub(crate) struct InitCmd {
  #[arg(long, default_value = "dmc.toml")]
  pub path: PathBuf,
}

impl InitCmd {
  /// `dmc init`: scaffold a default `dmc.toml` at `path` (refuses to
  /// overwrite existing files).
  pub(crate) fn run(self) -> std::io::Result<()> {
    if self.path.exists() {
      eprintln!("refusing to overwrite existing {}", self.path.display());
      std::process::exit(2);
    }
    let default = r#"output_dir = ".dmc"

[[collections]]
name = "docs"
pattern = "docs/**/*.mdx"
base_dir = "."
"#;
    std::fs::write(&self.path, default)?;
    println!("wrote {}", self.path.display());
    Ok(())
  }
}
