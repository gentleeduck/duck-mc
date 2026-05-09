use serde_json::{Map, Value, json};
use std::path::{Component, Path, PathBuf};

use crate::engine::{compile::CompileOutput, config::EngineConfig};

/// Build outcome: per-collection summaries plus non-fatal errors.
#[derive(Debug, Default)]
pub struct EngineReport {
  pub collections: Vec<CollectionReport>,
  pub errors: Vec<EngineError>,
}

/// One non-fatal compile failure (read fail, schema reject, ...). The
/// build continues unless `EngineConfig.strict` is set.
#[derive(Debug)]
pub struct EngineError {
  pub file: PathBuf,
  pub message: String,
}

/// Per-collection summary: record count and output path.
#[derive(Debug, Default)]
pub struct CollectionReport {
  pub name: String,
  pub records: usize,
  pub output_path: PathBuf,
}

/// `dmc_schema::Ctx` from a compiled doc. What schema `transform`/`refine`
/// predicates see (HTML, MDX body, TOC, plain text, path, ...).
pub fn build_schema_ctx(path: &Path, root: &Path, compiled: &CompileOutput, cfg: &EngineConfig) -> dmc_schema::Ctx {
  let mut ctx = dmc_schema::Ctx::new(path.to_path_buf(), root.to_path_buf(), compiled.content.clone());
  ctx.html = Some(compiled.html.clone());
  ctx.mdx_body = Some(compiled.body.clone());
  ctx.toc = Some(serde_json::to_value(&compiled.toc).unwrap_or(Value::Array(vec![])));
  ctx.plain_text = Some(compiled.excerpt.clone());
  if let (Some(dir), Some(base)) = (&cfg.compile.output_assets, &cfg.compile.output_base) {
    let mut p = dmc_schema::AssetPipeline::new(dir.into(), base.into());
    if let Some(t) = &cfg.output_name {
      p.name_template = t.into();
    }
    ctx.assets = Some(p);
  }
  ctx
}

/// Pack one compiled doc into a velite-shaped JSON record:
/// `{ ...frontmatter, code, raw, slug, permalink, path, ...optional html }`.
pub fn build_velite_record(
  compiled: CompileOutput,
  frontmatter: Value,
  path: &Path,
  base: &Path,
  collection: &str,
  include_html: bool,
) -> Value {
  let rel = path.strip_prefix(base).unwrap_or(path);
  let rel_str = rel.to_string_lossy().to_string();
  let source_file_path = path.to_string_lossy().to_string();
  let source_file_name = path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
  let source_file_dir = path
    .parent()
    .map(|p| {
      let mut comps: Vec<String> = p.components().map(|c| c.as_os_str().to_string_lossy().to_string()).collect();
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

/// Wrap the raw MDX body in an ES-module shell, hoisting frontmatter
/// imports above the function. Consumers `import` the default export.
pub fn wrap_mdx_module(body: &str, imports: &[String]) -> String {
  // Strip user imports from the body - they re-emit at module scope.
  let mut stripped = body.to_string();
  for imp in imports {
    let trimmed = imp.trim_end_matches('\n');
    if !trimmed.is_empty() {
      stripped = stripped.replacen(trimmed, "", 1);
    }
  }
  // Strip the trailing default-export literal; the module shell re-emits
  // its own. Falls back to the legacy direct-invoke return form.
  let mut stripped = stripped.trim_end_matches('\n').to_string();
  if let Some(idx) = stripped.rfind("return { default:") {
    stripped.truncate(idx);
  } else {
    stripped = stripped
      .trim_end_matches("return _createMdxContent(arguments[0]);")
      .trim_end_matches("return _createMdxContent(arguments[0])")
      .to_string();
  }
  let stripped = stripped.trim_end().to_string();
  // Replace `arguments[0]` references inside the function body with the
  // module-scoped __runtime constant.
  let stripped = stripped.replace("arguments[0]", "__runtime");

  let mut out = String::new();
  out.push_str("import { Fragment as _Fragment, jsx as _jsx, jsxs as _jsxs } from 'react/jsx-runtime'\n");
  for i in imports {
    out.push_str(i);
    if !i.ends_with('\n') {
      out.push('\n');
    }
  }
  out.push_str("const __runtime = { Fragment: _Fragment, jsx: _jsx, jsxs: _jsxs };\n");
  out.push_str(&stripped);
  out.push_str("\nexport default function MDXContent(props) { return _createMdxContent(props); }\n");
  out
}

/// Best-effort JS minifier: strips comments and collapses whitespace runs
/// into a single space. Not a full parser; regex literals, multi-line
/// strings, and JSX edge cases are not handled.
pub fn minify_js(src: &str) -> String {
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

/// Velite's permalink algorithm: collection name + `slug` (or file stem
/// when no slug). Returns the public URL for one record.
fn velite_permalink(abs: &str, rel: &str, collection: &str) -> String {
  let lc = collection.to_lowercase();
  let needle = format!("/{lc}/");
  let after = if let Some(idx) = abs.rfind(&needle) { &abs[idx + needle.len()..] } else { rel };
  after.trim_end_matches(".mdx").trim_end_matches(".md").to_string()
}

/// True when `s` is a bare JS identifier (safe to emit unquoted).
pub fn is_js_ident(s: &str) -> bool {
  let mut chars = s.chars();
  match chars.next() {
    Some(c) if c.is_ascii_alphabetic() || c == '_' || c == '$' => {},
    _ => return false,
  }
  chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
}

/// `kebab-case` / `snake_case` / `space case` -> `PascalCase`. Empty
/// input -> `"Doc"` so the caller can always concatenate a suffix.
pub fn pascal_case(name: &str) -> String {
  let mut out = String::with_capacity(name.len());
  let mut upper = true;
  for ch in name.chars() {
    if ch == '-' || ch == '_' || ch == ' ' {
      upper = true;
      continue;
    }
    if upper {
      out.extend(ch.to_uppercase());
      upper = false;
    } else {
      out.push(ch);
    }
  }
  if out.is_empty() { "Doc".into() } else { out }
}

/// POSIX-style relative path from `from_dir` to `target`. Canonicalises
/// when possible; always emits forward slashes (TS/ESM specifier shape).
pub fn relative_from(from_dir: &Path, target: &Path) -> String {
  let from_abs = from_dir.canonicalize().unwrap_or_else(|_| from_dir.to_path_buf());
  let to_abs = target.canonicalize().unwrap_or_else(|_| target.to_path_buf());
  let from_parts: Vec<Component<'_>> = from_abs.components().collect();
  let to_parts: Vec<Component<'_>> = to_abs.components().collect();
  let common = from_parts.iter().zip(&to_parts).take_while(|(a, b)| a == b).count();
  let ups = from_parts.len().saturating_sub(common);
  let mut out = String::new();
  for _ in 0..ups {
    out.push_str("../");
  }
  if ups == 0 {
    out.push_str("./");
  }
  let tail: Vec<String> = to_parts[common..]
    .iter()
    .filter_map(|c| match c {
      Component::Normal(s) => Some(s.to_string_lossy().into_owned()),
      _ => None,
    })
    .collect();
  out.push_str(&tail.join("/"));
  out
}
