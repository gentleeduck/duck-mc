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
        #[arg(long, default_value = "duck-md.toml")]
        config: PathBuf,
        #[arg(short, long)]
        strict: bool,
        #[arg(long)]
        clean: bool,
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
    /// Watch the content roots and rebuild on change.
    Dev {
        #[arg(long, default_value = "duck-md.toml")]
        config: PathBuf,
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
        Cmd::Build { config, strict, clean } => cmd_build(config, strict, clean),
        Cmd::Init { path } => cmd_init(path),
        Cmd::Compile { path } => cmd_compile(path),
        Cmd::Dev { config } => cmd_dev(config),
    }
}

fn load_engine_cfg(config: &PathBuf) -> std::io::Result<EngineConfig> {
    let raw = std::fs::read_to_string(config)?;
    let cfg: ConfigFile = toml::from_str(&raw)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
    Ok(EngineConfig {
        output_dir: cfg.output_dir,
        root: PathBuf::from("."),
        collections: cfg.collections.into_iter().map(|c| CollectionConfig {
            name: c.name,
            pattern: c.pattern,
            base_dir: c.base_dir,
            ..Default::default()
        }).collect(),
        ..Default::default()
    })
}

fn cmd_dev(config: PathBuf) -> std::io::Result<()> {
    use notify_debouncer_mini::{new_debouncer, notify::RecursiveMode};
    use std::sync::mpsc::channel;
    use std::time::{Duration, Instant};

    let cfg = load_engine_cfg(&config)?;
    print_build_result(duck_md::run(&cfg)?);

    let (tx, rx) = channel();
    let mut deb = new_debouncer(Duration::from_millis(50), tx)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
    let mut roots: Vec<PathBuf> = cfg.collections.iter().map(|c| c.base_dir.clone()).collect();
    roots.push(config.clone());
    for r in &roots {
        if r.exists() {
            deb.watcher().watch(r, RecursiveMode::Recursive)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        }
    }
    println!("👀 watching {} root(s) — Ctrl-C to stop", roots.len());
    while let Ok(events) = rx.recv() {
        if events.is_err() { continue; }
        let start = Instant::now();
        let cfg = match load_engine_cfg(&config) {
            Ok(c) => c,
            Err(e) => { eprintln!("config reload failed: {e}"); continue; }
        };
        match duck_md::run(&cfg) {
            Ok(rep) => {
                println!("\n↻ rebuilt in {:?}", start.elapsed());
                print_build_result(rep);
            }
            Err(e) => eprintln!("build error: {e}"),
        }
    }
    Ok(())
}

fn print_build_result(report: duck_md::EngineReport) {
    for c in &report.collections {
        println!("✓ {} — {} records → {}", c.name, c.records, c.output_path.display());
    }
    if !report.errors.is_empty() {
        for e in &report.errors {
            eprintln!("  \x1b[31m✗\x1b[0m {}: \x1b[2m{}\x1b[0m", e.file.display(), e.message);
        }
    }
}

fn cmd_build(config: PathBuf, strict: bool, clean: bool) -> std::io::Result<()> {
    let raw = std::fs::read_to_string(&config)?;
    let cfg: ConfigFile = toml::from_str(&raw)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
    let engine_cfg = EngineConfig {
        output_dir: cfg.output_dir,
        root: PathBuf::from("."),
        strict,
        clean,
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
        eprintln!();
        for e in &report.errors {
            eprintln!("  \x1b[31m✗\x1b[0m {}", e.file.display());
            eprintln!("    \x1b[2m{}\x1b[0m", e.message);
        }
        eprintln!("\n  \x1b[31m{}\x1b[0m validation error(s)", report.errors.len());
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
