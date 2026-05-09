//! Flamegraph of dmc native compile against the real `apps/duck`
//! preprocessed mirror — the same 370 mdx files the consumer build
//! processes. Captures the actual bottleneck distribution at the
//! scale the user cares about, not the toy fixture in
//! `flamegraph.rs`.
//!
//! Run:
//!   cargo run --release --example flamegraph_consumer --features pretty-code
//!
//! Output:
//!   duck-benchmarks/phase-6-correctness-cache/flamegraph/duck-ui.svg
//!   duck-benchmarks/phase-6-correctness-cache/flamegraph/duck-ui.txt
//!     (top-N self-time leaf-frame text summary)
//!
//! Like `flamegraph.rs`, uses pprof's signal-driven sampler so no
//! `perf_event_paranoid` toggle / sudo is needed.

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

/// Walk the apps/duck preprocessed mirror, return `(rel_path, source)`
/// pairs. The consumer-side preMdx pipeline writes into this dir; the
/// native build reads from it. Same input the production build sees.
fn load_corpus() -> Vec<(String, String)> {
  let candidates = [
    "/run/media/wildduck/duck1/wildduck/@duck/@duck-ui/apps/duck/content/.dmc-cache/preprocessed",
    // Fallbacks for other checkouts; first match wins.
    "../@duck-ui/apps/duck/content/.dmc-cache/preprocessed",
    "../../@duck-ui/apps/duck/content/.dmc-cache/preprocessed",
  ];
  let root = candidates
    .iter()
    .map(PathBuf::from)
    .find(|p| p.exists())
    .expect("apps/duck preprocessed mirror not found — run `bun run build:docs` once first");

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
  out
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

fn main() {
  let out_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .parent()
    .unwrap()
    .join("duck-benchmarks/phase-6-correctness-cache/flamegraph");
  fs::create_dir_all(&out_dir).unwrap();
  let svg_path = out_dir.join("duck-ui.svg");
  let txt_path = out_dir.join("duck-ui.txt");

  let corpus = load_corpus();
  eprintln!("loaded {} mdx files from apps/duck mirror", corpus.len());

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
  // Loop over the whole corpus until we've sampled ~5 s. At ~50 ms/pass
  // that's ~100 passes × 370 files = 37k compiles — plenty of stacks.
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
     corpus       : {} mdx files (apps/duck preprocessed mirror)\n\
     full passes  : {}\n\
     compiles     : {}\n\
     wall-clock   : {:.2}s\n\
     sampling     : pprof @ 997 Hz, signal-driven (no perf, no sudo)\n\
     \n\
     Open `duck-ui.svg` in a browser. Click frames to zoom; click the\n\
     title bar to reset. Search bar in the top-right.\n",
    corpus.len(),
    full_passes,
    files_compiled,
    elapsed.as_secs_f32(),
  );
  fs::write(&txt_path, summary).expect("write txt");

  let f = fs::File::create(&svg_path).expect("open svg");
  report.flamegraph(f).expect("write svg");

  eprintln!("compiled {files_compiled} files across {full_passes} full passes in {:.2}s", elapsed.as_secs_f32());
  eprintln!("wrote {}", svg_path.display());
  eprintln!("wrote {}", txt_path.display());
}
