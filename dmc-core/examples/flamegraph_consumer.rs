//! Flamegraph of dmc native compile against the real `apps/duck`
//! corpus - the ~370 mdx files the consumer build processes. Captures
//! the bottleneck distribution at the scale the user cares about, not
//! the toy fixture in `flamegraph.rs`. Prefers the preprocessed
//! mirror (`content/.dmc-cache/preprocessed`); falls back to the raw
//! `content/` tree when that hasn't been generated.
//!
//! Run:
//!   cargo run --release --example flamegraph_consumer --features pretty-code
//!
//! Output:
//!   duck-benchmarks/phase-7-g-hardening/flamegraph/duck-ui.svg
//!   duck-benchmarks/phase-7-g-hardening/flamegraph/duck-ui.txt
//!     (top-N self-time leaf-frame text summary)
//!
//! Like `flamegraph.rs`, uses pprof's signal-driven sampler so no
//! `perf_event_paranoid` toggle / sudo is needed - and like it, this is
//! Unix only (pprof relies on POSIX signals / `nix`); on Windows it
//! compiles to a stub `main`.

#[cfg(not(unix))]
fn main() {
  eprintln!("`flamegraph_consumer` example needs pprof's signal-driven sampler; supported on Unix only.");
}

#[cfg(unix)]
fn main() {
  imp::run();
}

#[cfg(unix)]
mod imp {
  use dmc_codegen::{HtmlEmitter, MdxBodyEmitter, NodeSink, Walker};
  use dmc_diagnostic::Code;
  use dmc_diagnostic::metadata::{Origin, SourceMeta};
  use dmc_lexer::Lexer;
  use dmc_parser::Parser;
  use dmc_transform::{Pipeline, PipelineConfig};
  use duck_diagnostic::DiagnosticEngine;
  use pprof::ProfilerGuardBuilder;
  use std::fs;
  use std::path::PathBuf;
  use std::sync::Arc;
  use std::time::Instant;

  /// Walk the apps/duck mdx corpus, return `(rel_path, source)` pairs.
  /// Prefers the preprocessed mirror (`.dmc-cache/preprocessed`) - the
  /// exact input the production native build sees - and falls back to the
  /// raw `content/` tree when that mirror hasn't been generated yet
  /// (`bun run build:docs` writes it). Raw vs preprocessed differs only
  /// by the JS preMdx pass, which doesn't touch the native compile path
  /// this flamegraph profiles.
  fn load_corpus() -> (Vec<(String, String)>, &'static str) {
    // (path, kind) - first existing wins.
    let candidates = [
      (
        "/run/media/wildduck/duck1/wildduck/@duck/@duck-ui/apps/duck/content/.dmc-cache/preprocessed",
        "preprocessed mirror",
      ),
      ("../@duck-ui/apps/duck/content/.dmc-cache/preprocessed", "preprocessed mirror"),
      ("../../@duck-ui/apps/duck/content/.dmc-cache/preprocessed", "preprocessed mirror"),
      // Raw-content fallback (no preMdx mirror generated yet).
      ("/run/media/wildduck/duck1/wildduck/@duck/@duck-ui/apps/duck/content", "raw content tree (no preMdx pass)"),
      ("../@duck-ui/apps/duck/content", "raw content tree (no preMdx pass)"),
      ("../../@duck-ui/apps/duck/content", "raw content tree (no preMdx pass)"),
    ];
    let (root, kind) = candidates
      .iter()
      .map(|(p, k)| (PathBuf::from(p), *k))
      .find(|(p, _)| p.exists())
      .expect("apps/duck corpus not found - checkout @duck-ui beside this repo, or run `bun run build:docs` once");

    let mut out = Vec::new();
    let mut stack = vec![root.clone()];
    while let Some(d) = stack.pop() {
      for ent in fs::read_dir(&d).unwrap_or_else(|e| panic!("read_dir {}: {e}", d.display())) {
        let Ok(ent) = ent else { continue };
        let p = ent.path();
        if p.is_dir() {
          stack.push(p);
          continue;
        }
        let Some(ext) = p.extension().and_then(|s| s.to_str()) else { continue };
        if ext != "mdx" && ext != "md" {
          continue;
        }
        let rel = p.strip_prefix(&root).unwrap().to_string_lossy().into_owned();
        let src = fs::read_to_string(&p).unwrap_or_default();
        if !src.is_empty() {
          out.push((rel, src));
        }
      }
    }
    (out, kind)
  }

  fn compile_one(rel: &str, source: &str, pipeline: &Pipeline) {
    let meta = Arc::new(SourceMeta { path: Arc::from(rel), origin: Origin::Inline(rel.to_string().leak()) });
    let mut diag = DiagnosticEngine::<Code>::new();
    let mut lex = Lexer::new(source, meta.clone(), &mut diag);
    let _ = lex.scan_tokens();
    let mut doc = {
      let mut parser = Parser::new(lex.tokens, meta.clone(), &mut diag);
      parser.parse()
    };
    pipeline.run(&mut doc, &meta, &mut diag);
    let mut html = HtmlEmitter::new();
    let mut body = MdxBodyEmitter::new();
    let mut sinks: Vec<&mut dyn NodeSink> = vec![&mut html, &mut body];
    Walker::new(&doc).walk(sinks.as_mut_slice());
    let _ = html.into_parts();
    let _ = body.into_parts();
  }

  pub fn run() {
    let out_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
      .parent()
      .unwrap()
      .join("duck-benchmarks/phase-7-g-hardening/flamegraph");
    fs::create_dir_all(&out_dir).unwrap();
    let svg_path = out_dir.join("duck-ui.svg");
    let txt_path = out_dir.join("duck-ui.txt");

    let (corpus, corpus_kind) = load_corpus();
    eprintln!("loaded {} mdx files from apps/duck {corpus_kind}", corpus.len());

    let pipeline_cfg = PipelineConfig::default();
    let pipeline = Pipeline::with_defaults_for(&pipeline_cfg);

    let guard = ProfilerGuardBuilder::default()
      .frequency(997)
      .blocklist(&["libc", "libgcc", "pthread", "vdso"])
      .build()
      .expect("pprof guard");

    let started = Instant::now();
    let mut full_passes = 0u64;
    let mut files_compiled = 0u64;
    // Loop over the whole corpus until we've sampled ~5 s. The check is
    // between passes, so a single slow pass can overshoot - on the raw
    // content tree (heavy MDX/JSX recovery, big fenced code -> syntect)
    // one pass already runs well past 5 s and is plenty of stacks.
    while started.elapsed().as_secs_f32() < 5.0 {
      for (rel, src) in &corpus {
        compile_one(rel, src, &pipeline);
        files_compiled += 1;
      }
      full_passes += 1;
    }
    let elapsed = started.elapsed();

    let report = guard.report().build().expect("build report");

    let summary = format!(
      "duck-ui flamegraph fixture\n\
       -------------------------\n\
       corpus       : {} mdx files (apps/duck {})\n\
       full passes  : {}\n\
       compiles     : {}\n\
       wall-clock   : {:.2}s ({:.1} ms/file avg over all compiles)\n\
       sampling     : pprof @ 997 Hz, signal-driven (no perf, no sudo)\n\
       \n\
       Open `duck-ui.svg` in a browser. Click frames to zoom; click the\n\
       title bar to reset. Search bar in the top-right.\n",
      corpus.len(),
      corpus_kind,
      full_passes,
      files_compiled,
      elapsed.as_secs_f32(),
      elapsed.as_secs_f64() * 1000.0 / files_compiled.max(1) as f64,
    );
    fs::write(&txt_path, summary).expect("write txt");

    let f = fs::File::create(&svg_path).expect("open svg");
    report.flamegraph(f).expect("write svg");

    eprintln!("compiled {files_compiled} files across {full_passes} full passes in {:.2}s", elapsed.as_secs_f32());
    eprintln!("wrote {}", svg_path.display());
    eprintln!("wrote {}", txt_path.display());
  }
}
