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
    let ext = config.extension().and_then(|s| s.to_str()).unwrap_or("");
    if matches!(ext, "ts" | "js" | "mjs") {
        return load_ts_config(config);
    }
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

fn load_ts_config(config: &PathBuf) -> std::io::Result<EngineConfig> {
    use std::io::Write;
    let abs = std::fs::canonicalize(config)?;
    let script = include_str!("../scripts/load-config.mjs");
    let mut tmp = tempfile::Builder::new().suffix(".mjs").tempfile()?;
    tmp.write_all(script.as_bytes())?;
    tmp.flush()?;
    let tmp_path = tmp.path().to_path_buf();

    let attempts: &[(&str, &[&str])] = &[
        ("bun", &[]),
        ("node", &["--import", "tsx"]),
    ];
    let mut last_err: Option<String> = None;
    for (cmd, prefix_args) in attempts {
        let mut c = std::process::Command::new(cmd);
        c.args(*prefix_args).arg(&tmp_path).arg(&abs);
        match c.output() {
            Ok(out) if out.status.success() => {
                let json = String::from_utf8(out.stdout).map_err(|e|
                    std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
                let cfg: EngineConfig = serde_json::from_str(&json).map_err(|e|
                    std::io::Error::new(std::io::ErrorKind::InvalidData,
                        format!("ts config: {e}\n--- output ---\n{json}")))?;
                return Ok(cfg);
            }
            Ok(out) => {
                last_err = Some(format!("{cmd} exit {}: {}",
                    out.status, String::from_utf8_lossy(&out.stderr)));
            }
            Err(e) => last_err = Some(format!("{cmd}: {e}")),
        }
    }
    Err(std::io::Error::new(std::io::ErrorKind::NotFound, format!(
        "ts config requires `bun` or `node` w/ tsx on PATH ({})",
        last_err.unwrap_or_default(),
    )))
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
        let Ok(events) = events else { continue };
        let start = Instant::now();
        let cfg = match load_engine_cfg(&config) {
            Ok(c) => c,
            Err(e) => { eprintln!("config reload failed: {e}"); continue; }
        };
        // Determine which collections were affected. If config file changed, rebuild all.
        let touched_paths: Vec<_> = events.iter().map(|e| e.path.clone()).collect();
        let config_changed = touched_paths.iter().any(|p| p.canonicalize().ok() == config.canonicalize().ok());
        let scoped: Vec<_> = if config_changed {
            cfg.collections.iter().cloned().collect()
        } else {
            cfg.collections.iter().filter(|c| {
                let base = c.base_dir.canonicalize().unwrap_or_else(|_| c.base_dir.clone());
                touched_paths.iter().any(|p| {
                    let p_abs = p.canonicalize().unwrap_or_else(|_| p.clone());
                    p_abs.starts_with(&base)
                })
            }).cloned().collect()
        };
        if scoped.is_empty() { continue; }
        let scoped_cfg = duck_md::EngineConfig { collections: scoped, ..cfg };
        match duck_md::run(&scoped_cfg) {
            Ok(rep) => {
                println!("\n↻ rebuilt {} collection(s) in {:?}", rep.collections.len(), start.elapsed());
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
    let mut engine_cfg = load_engine_cfg(&config)?;
    if strict { engine_cfg.strict = true; }
    if clean { engine_cfg.clean = true; }
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
