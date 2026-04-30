use std::path::PathBuf;

use dmc::compile;

#[derive(clap::Args)]
pub(crate) struct CompileCmd {
  #[arg(long, default_value = "dmc.toml")]
  pub path: PathBuf,
}

impl CompileCmd {
  /// `dmc compile`: read one mdx file, run the default pipeline, print
  /// the resulting `CompileOutput` as pretty JSON to stdout.
  pub(crate) fn run(self) -> std::io::Result<()> {
    let src = std::fs::read_to_string(&self.path)?;
    let out = compile(&src);
    let json = serde_json::to_string_pretty(&out).unwrap();
    println!("{}", json);
    Ok(())
  }
}
