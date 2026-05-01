use serde_json::{Map, Value, json};
use std::path::{Path, PathBuf};

use crate::engine::config::EngineConfig;

/// Aggregated outcome of a build: per-collection summaries plus any
/// non-fatal errors encountered.
#[derive(Debug, Default)]
pub struct EngineReport {
  pub collections: Vec<CollectionReport>,
  pub errors: Vec<EngineError>,
}

/// Single non-fatal failure during compilation (read fail, schema reject,
/// etc.). The build continues unless `EngineConfig.strict` is set.
#[derive(Debug)]
pub struct EngineError {
  pub file: PathBuf,
  pub message: String,
}

/// Per-collection summary: how many records were emitted and where the
/// index file landed.
#[derive(Debug, Default)]
pub struct CollectionReport {
  pub name: String,
  pub records: usize,
  pub output_path: PathBuf,
}

/// Build a `dmc_schema::Ctx` from a compiled doc — this is what schema
/// `transform`/`refine` predicates see when validating frontmatter (HTML,
/// MDX body, TOC, plain text, file path, etc.).
pub fn build_schema_ctx(
  path: &Path,
  root: &Path,
  compiled: &crate::CompileOutput,
  cfg: &EngineConfig,
) -> dmc_schema::Ctx {
  let mut ctx =
    dmc_schema::Ctx::new(path.to_path_buf(), root.to_path_buf(), compiled.content.clone());
  ctx.html = Some(compiled.html.clone());
  ctx.mdx_body = Some(compiled.body.clone());
  ctx.toc = Some(serde_json::to_value(&compiled.toc).unwrap_or(Value::Array(vec![])));
  ctx.plain_text = Some(compiled.excerpt.clone());
  if let (Some(dir), Some(base)) = (&cfg.output_assets, &cfg.output_base) {
    let mut p = dmc_schema::AssetPipeline::new(dir.clone(), base.clone());
    if let Some(t) = &cfg.output_name {
      p.name_template = t.clone();
    }
    ctx.assets = Some(p);
  }
  ctx
}

/// Emit the top-level entry point that re-exports every collection.
/// `format` is `"esm"` (default) or `"cjs"` — picks `index.mjs` vs `index.js`.
fn write_index(out_dir: &Path, report: &EngineReport, format: &str) -> std::io::Result<()> {
  let mut js = String::new();
  if format == "cjs" {
    for c in &report.collections {
      js.push_str(&format!("exports.{name} = require('./{name}.json')\n", name = c.name));
    }
  } else {
    for c in &report.collections {
      js.push_str(&format!(
        "export {{ default as {name} }} from './{name}.json' with {{ type: 'json' }}\n",
        name = c.name
      ));
    }
  }
  std::fs::write(out_dir.join("index.js"), js)?;

  let mut dts = String::from(
    "export interface TocItem { title: string; url: string; items: TocItem[] }\n\
         export interface Metadata { readingTime: number; wordCount: number }\n\
         export interface DocRecord {\n\
         \u{20}\u{20}body: string\n\
         \u{20}\u{20}content: string\n\
         \u{20}\u{20}excerpt: string\n\
         \u{20}\u{20}metadata: Metadata\n\
         \u{20}\u{20}toc: TocItem[]\n\
         \u{20}\u{20}contentType: string\n\
         \u{20}\u{20}flattenedPath: string\n\
         \u{20}\u{20}permalink: string\n\
         \u{20}\u{20}slug: string\n\
         \u{20}\u{20}sourceFileDir: string\n\
         \u{20}\u{20}sourceFileName: string\n\
         \u{20}\u{20}sourceFilePath: string\n\
         \u{20}\u{20}[frontmatterField: string]: unknown\n\
         }\n",
  );
  for c in &report.collections {
    dts.push_str(&format!("export declare const {name}: DocRecord[]\n", name = c.name));
  }
  std::fs::write(out_dir.join("index.d.ts"), dts)?;
  Ok(())
}

/// Pack one compiled document into the velite-shaped JSON record:
/// `{ ...frontmatter, code, raw, slug, permalink, path, ...optional html }`.
pub fn build_velite_record(
  compiled: crate::CompileOutput,
  frontmatter: Value,
  path: &Path,
  base: &Path,
  collection: &str,
  include_html: bool,
) -> Value {
  let rel = path.strip_prefix(base).unwrap_or(path);
  let rel_str = rel.to_string_lossy().to_string();
  let source_file_path = path.to_string_lossy().to_string();
  let source_file_name =
    path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
  let source_file_dir = path
    .parent()
    .map(|p| {
      let mut comps: Vec<String> =
        p.components().map(|c| c.as_os_str().to_string_lossy().to_string()).collect();
      if comps.len() >= 2 {
        let last2 = comps.split_off(comps.len() - 2);
        last2.join("/")
      } else {
        comps.join("/")
      }
    })
    .unwrap_or_default();
  let content_type = path.extension().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
  let permalink = velite_permalink(&source_file_path, &rel_str, collection);
  let flattened_path = permalink.clone();
  let slug = if permalink.is_empty() {
    collection.to_lowercase()
  } else {
    format!("{}/{}", collection.to_lowercase(), permalink)
  };

  let mut map: Map<String, Value> = Map::new();
  if let Value::Object(fm) = frontmatter {
    for (k, v) in fm {
      map.insert(k, v);
    }
  }

  map.insert("body".into(), Value::String(compiled.body));
  map.insert("content".into(), Value::String(compiled.content));
  if include_html {
    map.insert("html".into(), Value::String(compiled.html.clone()));
  }
  map.insert("excerpt".into(), Value::String(compiled.excerpt));
  map.insert("metadata".into(), serde_json::to_value(&compiled.metadata).unwrap_or(json!({})));
  map.insert("toc".into(), serde_json::to_value(&compiled.toc).unwrap_or(Value::Array(vec![])));
  map.insert("contentType".into(), Value::String(content_type));
  map.insert("flattenedPath".into(), Value::String(flattened_path));
  map.insert("permalink".into(), Value::String(permalink));
  map.insert("slug".into(), Value::String(slug));
  map.insert("sourceFileDir".into(), Value::String(source_file_dir));
  map.insert("sourceFileName".into(), Value::String(source_file_name));
  map.insert("sourceFilePath".into(), Value::String(source_file_path));

  Value::Object(map)
}

/// Wrap the raw MDX body string in an ES-module shell so consumers can
/// `import { default as Content } from "./post.mjs"` and render it
/// directly. Hoists frontmatter imports above the function definition.
fn wrap_mdx_module(body: &str, imports: &[String]) -> String {
  // Strip user imports from the body — they re-emit at module scope.
  let mut stripped = body.to_string();
  for imp in imports {
    let trimmed = imp.trim_end_matches('\n');
    if !trimmed.is_empty() {
      stripped = stripped.replacen(trimmed, "", 1);
    }
  }
  // Strip the trailing factory invocation; we'll re-call it ourselves.
  let stripped = stripped
    .trim_end_matches('\n')
    .trim_end_matches("return _createMdxContent(arguments[0]);")
    .trim_end_matches("return _createMdxContent(arguments[0])")
    .trim_end()
    .to_string();
  // Replace `arguments[0]` references inside the function body with the
  // module-scoped __runtime constant.
  let stripped = stripped.replace("arguments[0]", "__runtime");

  let mut out = String::new();
  out.push_str(
    "import { Fragment as _Fragment, jsx as _jsx, jsxs as _jsxs } from 'react/jsx-runtime'\n",
  );
  for i in imports {
    out.push_str(i);
    if !i.ends_with('\n') {
      out.push('\n');
    }
  }
  out.push_str("const __runtime = { Fragment: _Fragment, jsx: _jsx, jsxs: _jsxs };\n");
  out.push_str(&stripped);
  out
    .push_str("\nexport default function MDXContent(props) { return _createMdxContent(props); }\n");
  out
}

/// Best-effort JS minifier: strip comments, collapse whitespace, drop
/// blank lines. Tiny + safe — not a full parser, so corner cases (regex
/// literals, multi-line strings) are handled by skipping over them.
fn minify_js(src: &str) -> String {
  #[derive(Clone, Copy, PartialEq)]
  enum St {
    Code,
    Squote,
    Dquote,
    Btick,
    LineComment,
    BlockComment,
  }
  let mut out = String::with_capacity(src.len());
  let mut st = St::Code;
  let mut prev_ws = false;
  let mut chars = src.chars().peekable();
  while let Some(c) = chars.next() {
    match st {
      St::Code => {
        if c == '/' {
          if matches!(chars.peek(), Some('/')) {
            chars.next();
            st = St::LineComment;
            continue;
          }
          if matches!(chars.peek(), Some('*')) {
            chars.next();
            st = St::BlockComment;
            continue;
          }
        }
        if c == '"' {
          st = St::Dquote;
          out.push(c);
          prev_ws = false;
          continue;
        }
        if c == '\'' {
          st = St::Squote;
          out.push(c);
          prev_ws = false;
          continue;
        }
        if c == '`' {
          st = St::Btick;
          out.push(c);
          prev_ws = false;
          continue;
        }
        if c == '\n' || c == '\t' || c == ' ' {
          if prev_ws {
            continue;
          }
          prev_ws = true;
          out.push(' ');
          continue;
        }
        prev_ws = false;
        out.push(c);
      },
      St::Squote => {
        out.push(c);
        if c == '\\' {
          if let Some(n) = chars.next() {
            out.push(n);
          }
          continue;
        }
        if c == '\'' {
          st = St::Code;
        }
      },
      St::Dquote => {
        out.push(c);
        if c == '\\' {
          if let Some(n) = chars.next() {
            out.push(n);
          }
          continue;
        }
        if c == '"' {
          st = St::Code;
        }
      },
      St::Btick => {
        out.push(c);
        if c == '\\' {
          if let Some(n) = chars.next() {
            out.push(n);
          }
          continue;
        }
        if c == '`' {
          st = St::Code;
        }
      },
      St::LineComment => {
        if c == '\n' {
          st = St::Code;
        }
      },
      St::BlockComment => {
        if c == '*' && matches!(chars.peek(), Some('/')) {
          chars.next();
          st = St::Code;
        }
      },
    }
  }
  out
}

/// True when the user configured any remark/rehype plugin — triggers the
/// Node-side sidecar pipeline.
pub fn has_js_plugins(cfg: &EngineConfig) -> bool {
  // Match velite: when output.html is requested, run the JS pipeline so
  // user gets gfm + comment-strip + their plugins. Or when any plugin list
  // is non-empty.

  let any_filled = |v: &Vec<Value>| !v.is_empty();
  any_filled(&cfg.markdown_remark_plugins)
    || any_filled(&cfg.markdown_rehype_plugins)
    || any_filled(&cfg.mdx_remark_plugins)
    || any_filled(&cfg.mdx_rehype_plugins)
}

/// Spawn a Node sidecar (`node` on PATH) that loads the user's remark /
/// rehype plugin chains and re-renders the markdown to HTML. `None` when
/// the sidecar is unavailable or fails — the build falls back to native
/// codegen output.
fn run_sidecar(markdown: &str, cfg: &EngineConfig) -> Option<String> {
  use std::io::Write;
  use std::process::{Command, Stdio};
  let entry = std::env::var("dmc_SIDECAR").ok().or_else(|| Some("dmc-sidecar/index.mjs".into()))?;
  let merge = |a: &Vec<Value>, b: &Vec<Value>| -> Value {
    Value::Array(a.iter().chain(b.iter()).cloned().collect())
  };
  let req = json!({
      "markdown": markdown,
      "remarkPlugins": merge(&cfg.markdown_remark_plugins, &cfg.mdx_remark_plugins),
      "rehypePlugins": merge(&cfg.markdown_rehype_plugins, &cfg.mdx_rehype_plugins),
  });
  let mut child = Command::new("node")
    .arg(&entry)
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::null())
    .spawn()
    .ok()?;
  child.stdin.as_mut()?.write_all(req.to_string().as_bytes()).ok()?;
  let out = child.wait_with_output().ok()?;
  if !out.status.success() {
    return None;
  }
  let parsed: Value = serde_json::from_slice(&out.stdout).ok()?;
  parsed.get("html").and_then(|v| v.as_str()).map(String::from)
}

/// Mirror velite's permalink algorithm: collection name + `slug` (or the
/// file stem when no slug exists). Produces the public URL for one record.
fn velite_permalink(abs: &str, rel: &str, collection: &str) -> String {
  let lc = collection.to_lowercase();
  let needle = format!("/{lc}/");
  let after = if let Some(idx) = abs.rfind(&needle) { &abs[idx + needle.len()..] } else { rel };
  after.trim_end_matches(".mdx").trim_end_matches(".md").to_string()
}
