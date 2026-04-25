use std::path::PathBuf;
use clap::{Parser, Subcommand};
use duck_md::{run, EngineConfig, CollectionConfig, compile};

#[derive(Parser)]
#[command(name = "duck-md", version, about = "Rust MDX compiler — drop-in for velite docs role")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Build all collections from duck-md.toml.
    Build {
        /// Path to config file
        #[arg(long, default_value = "duck-md.toml")]
        config: PathBuf,
    },
    /// Scaffold a default duck-md.toml in the current directory.
    Init {
        #[arg(long, default_value = "duck-md.toml")]
        path: PathBuf,
    },
    /// Compile a single MDX file and print JSON to stdout.
    Compile {
        path: PathBuf,
    },
}

#[derive(serde::Serialize, serde::Deserialize)]
struct ConfigFile {
    output_dir: PathBuf,
    collections: Vec<CollectionEntry>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct CollectionEntry {
    name: String,
    pattern: String,
    base_dir: PathBuf,
}

fn main() -> std::io::Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Build { config } => cmd_build(config),
        Cmd::Init { path } => cmd_init(path),
        Cmd::Compile { path } => cmd_compile(path),
    }
}

fn cmd_build(config: PathBuf) -> std::io::Result<()> {
    let raw = std::fs::read_to_string(&config)?;
    let cfg: ConfigFile = toml::from_str(&raw)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
    let engine_cfg = EngineConfig {
        output_dir: cfg.output_dir,
        root: PathBuf::from("."),
        collections: cfg.collections.into_iter().map(|c| CollectionConfig {
            name: c.name,
            pattern: c.pattern,
            base_dir: c.base_dir,
            ..Default::default()
        }).collect(),
        ..Default::default()
    };
    let report = run(&engine_cfg)?;
    for c in &report.collections {
        println!("✓ {} — {} records → {}", c.name, c.records, c.output_path.display());
    }
    if !report.errors.is_empty() {
        eprintln!("\n{} validation error(s):", report.errors.len());
        for e in &report.errors {
            eprintln!("  {}: {}", e.file.display(), e.message);
        }
    }
    Ok(())
}

fn cmd_init(path: PathBuf) -> std::io::Result<()> {
    if path.exists() {
        eprintln!("refusing to overwrite existing {}", path.display());
        std::process::exit(2);
    }
    let default = r#"output_dir = ".duck-md"

[[collections]]
name = "docs"
pattern = "docs/**/*.mdx"
base_dir = "."
"#;
    std::fs::write(&path, default)?;
    println!("wrote {}", path.display());
    Ok(())
}

fn cmd_compile(path: PathBuf) -> std::io::Result<()> {
    let src = std::fs::read_to_string(&path)?;
    let out = compile(&src);
    let json = serde_json::to_string_pretty(&out).unwrap();
    println!("{}", json);
    Ok(())
}
