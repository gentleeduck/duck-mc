// //! Drive the full dmc-core orchestrator. Two modes:
// //!
// //!   compile mode (default) — one file → `CompileOutput`
// //!     cargo run -p dmc-core --bin core
// //!     cargo run -p dmc-core --bin core -- ../samples/headings.mdx
// //!     echo '# hi' | cargo run -p dmc-core --bin core -- -
// //!     cargo run -p dmc-core --bin core -- --field html
// //!     cargo run -p dmc-core --bin core -- --json
// //!
// //!   engine mode — multi-file build via a synthesized config + temp output
// //!     cargo run -p dmc-core --bin core -- --engine
// //!     cargo run -p dmc-core --bin core -- --engine --keep
// //!
// //! Flags:
// //!   --engine          run the multi-file engine over `samples/` instead of compile()
// //!   --field <name>    print only one CompileOutput field (frontmatter, content,
// //!                     html, body, excerpt, metadata, toc, imports, exports)
// //!   --json            full structured CompileOutput / EngineReport as pretty JSON
// //!   --keep            engine mode: leave the temp output dir on disk + print path
// //!   --quiet           suppress trailing summary line
//
// use dmc::{CollectionConfig, CompileOutput, EngineConfig, compile, run};
// use std::io::{self, Read};
// use std::path::PathBuf;
//
// fn main() -> io::Result<()> {
//   let mut args: Vec<String> = std::env::args().skip(1).collect();
//   let engine_mode = take_flag(&mut args, "--engine");
//   let show_json = take_flag(&mut args, "--json");
//   let quiet = take_flag(&mut args, "--quiet");
//   let keep = take_flag(&mut args, "--keep");
//   let field = take_value(&mut args, "--field");
//
//   if engine_mode {
//     return run_engine_mode(show_json, keep, quiet);
//   }
//
//   let (label, source) = read_source(args.first().map(String::as_str))?;
//   let out = compile(&source);
//
//   if show_json {
//     let json = serde_json::to_string_pretty(&out).unwrap_or_else(|_| "{}".into());
//     println!("{}", json);
//     return Ok(());
//   }
//
//   if let Some(name) = field.as_deref() {
//     print_field(&out, name);
//     return Ok(());
//   }
//
//   print_summary(&label, &out);
//
//   if !quiet {
//     println!(
//       "\nsummary: {} word(s), {} min read, {} toc item(s), {} import(s), {} export(s)",
//       out.metadata.word_count,
//       out.metadata.reading_time,
//       out.toc.len(),
//       out.imports.len(),
//       out.exports.len(),
//     );
//   }
//   Ok(())
// }
//
// fn read_source(arg: Option<&str>) -> io::Result<(String, String)> {
//   match arg {
//     None => {
//       let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../samples/index.mdx");
//       let src = std::fs::read_to_string(&path)?;
//       let label = path.file_name().unwrap().to_string_lossy().into_owned();
//       Ok((label, src))
//     },
//     Some("-") => {
//       let mut buf = String::new();
//       io::stdin().read_to_string(&mut buf)?;
//       Ok(("<stdin>".to_string(), buf))
//     },
//     Some(p) => {
//       let path = PathBuf::from(p);
//       let src = std::fs::read_to_string(&path)?;
//       let label = path.file_name().unwrap().to_string_lossy().into_owned();
//       Ok((label, src))
//     },
//   }
// }
//
// fn print_summary(label: &str, out: &CompileOutput) {
//   println!("== {} ==", label);
//
//   println!("\n-- frontmatter --");
//   println!("{}", serde_json::to_string_pretty(&out.frontmatter).unwrap_or_else(|_| "null".into()));
//
//   println!("\n-- metadata --");
//   println!("words: {}, reading_time: {} min", out.metadata.word_count, out.metadata.reading_time);
//
//   println!("\n-- excerpt --");
//   println!("{}", out.excerpt);
//
//   if !out.imports.is_empty() {
//     println!("\n-- imports ({}) --", out.imports.len());
//     for i in &out.imports {
//       println!("  {}", i.trim());
//     }
//   }
//   if !out.exports.is_empty() {
//     println!("\n-- exports ({}) --", out.exports.len());
//     for e in &out.exports {
//       println!("  {}", e.trim());
//     }
//   }
//
//   if !out.toc.is_empty() {
//     println!("\n-- toc --");
//     print_toc(&out.toc, 0);
//   }
//
//   println!("\n-- html ({} bytes) --", out.html.len());
//   println!("{}", out.html);
//
//   println!("\n-- mdx body ({} bytes) --", out.body.len());
//   println!("{}", out.body);
// }
//
// fn print_toc(items: &[dmc::TocItem], depth: usize) {
//   for item in items {
//     println!("{}- {} ({})", "  ".repeat(depth), item.title, item.url);
//     print_toc(&item.items, depth + 1);
//   }
// }
//
// fn print_field(out: &CompileOutput, name: &str) {
//   match name {
//     "frontmatter" => println!(
//       "{}",
//       serde_json::to_string_pretty(&out.frontmatter).unwrap_or_else(|_| "null".into())
//     ),
//     "frontmatter_raw" | "frontmatterRaw" => println!("{}", out.frontmatter_raw),
//     "content" => println!("{}", out.content),
//     "html" => println!("{}", out.html),
//     "body" => println!("{}", out.body),
//     "excerpt" => println!("{}", out.excerpt),
//     "metadata" => {
//       println!("{}", serde_json::to_string_pretty(&out.metadata).unwrap_or_else(|_| "{}".into()))
//     },
//     "toc" => println!("{}", serde_json::to_string_pretty(&out.toc).unwrap_or_else(|_| "[]".into())),
//     "imports" => {
//       for i in &out.imports {
//         println!("{}", i.trim());
//       }
//     },
//     "exports" => {
//       for e in &out.exports {
//         println!("{}", e.trim());
//       }
//     },
//     other => {
//       eprintln!(
//         "unknown field {:?}. valid: frontmatter, frontmatter_raw, content, html, body, \
//          excerpt, metadata, toc, imports, exports",
//         other
//       );
//       std::process::exit(2);
//     },
//   }
// }
//
// fn run_engine_mode(show_json: bool, keep: bool, quiet: bool) -> io::Result<()> {
//   let samples_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../samples");
//   let temp = tempfile::tempdir()?;
//   let out_dir = temp.path().join("out");
//
//   let cfg = EngineConfig {
//     collections: vec![CollectionConfig {
//       name: "samples".to_string(),
//       pattern: "*.mdx".to_string(),
//       base_dir: samples_dir.clone(),
//       ..Default::default()
//     }],
//     output_dir: out_dir.clone(),
//     root: samples_dir.clone(),
//     include_html: true,
//     ..Default::default()
//   };
//
//   let report = run(&cfg)?;
//
//   if show_json {
//     let json = serde_json::json!({
//       "collections": report.collections.iter().map(|c| serde_json::json!({
//         "name": c.name,
//         "records": c.records,
//         "output_path": c.output_path,
//       })).collect::<Vec<_>>(),
//       "errors": report.errors.iter().map(|e| serde_json::json!({
//         "file": e.file,
//         "message": e.message,
//       })).collect::<Vec<_>>(),
//       "outputDir": out_dir,
//     });
//     println!("{}", serde_json::to_string_pretty(&json).unwrap_or_else(|_| "{}".into()));
//   } else {
//     println!("== engine mode ==");
//     println!("base: {}", samples_dir.display());
//     println!("output: {}", out_dir.display());
//     for c in &report.collections {
//       println!("  ✓ {} — {} record(s) → {}", c.name, c.records, c.output_path.display());
//       if let Ok(bytes) = std::fs::read_to_string(&c.output_path) {
//         let preview: String = bytes.chars().take(400).collect();
//         println!("    preview: {}{}", preview, if bytes.chars().count() > 400 { "…" } else { "" });
//       }
//     }
//     if !report.errors.is_empty() {
//       println!("\nerrors:");
//       for e in &report.errors {
//         println!("  ✗ {}: {}", e.file.display(), e.message);
//       }
//     }
//     if !quiet {
//       println!(
//         "\nsummary: {} collection(s), {} error(s)",
//         report.collections.len(),
//         report.errors.len()
//       );
//     }
//   }
//
//   if keep {
//     let kept = temp.keep();
//     println!("\nkept output dir: {}", kept.display());
//   }
//   Ok(())
// }
//
// fn take_flag(args: &mut Vec<String>, name: &str) -> bool {
//   let had = args.iter().any(|a| a == name);
//   args.retain(|a| a != name);
//   had
// }
//
// fn take_value(args: &mut Vec<String>, name: &str) -> Option<String> {
//   let mut i = 0;
//   while i < args.len() {
//     if args[i] == name && i + 1 < args.len() {
//       let v = args.remove(i + 1);
//       args.remove(i);
//       return Some(v);
//     }
//     if let Some(rest) = args[i].strip_prefix(&format!("{name}=")) {
//       let v = rest.to_string();
//       args.remove(i);
//       return Some(v);
//     }
//     i += 1;
//   }
//   None
// }

fn main() {}
