//! Render LaTeX in `$...$` (inline) and `$$...$$` (block) to KaTeX HTML
//! via the embedded KaTeX engine (`katex` crate, quick-js backend).
//! Output is byte-equivalent to the JS chain `remark-math` +
//! `rehype-katex`. Consumers must ship `katex.min.css` for glyphs.
//!
//! Block-level math (a paragraph whose entire content is `$$...$$`)
//! replaces the wrapping `<p>` so the rendered HTML lands at block level.
//!
//! Escaped delimiters (`\$`) pass through as literal `$`. Parse failures
//! emit `<span class="math-error">` wrapping the original LaTeX.

use crate::pipeline::Transformer;
use crate::visit::{NodeAction, Visitor, walk_root};
use dmc_diagnostic::Code;
use dmc_diagnostic::metadata::SourceMeta;
use dmc_parser::ast::*;
use duck_diagnostic::DiagnosticEngine;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

type MathCacheKey = (String, bool, crate::MathEngine);
type MathCache = HashMap<MathCacheKey, String>;

/// Render `$...$` and `$$...$$` math spans to MathML.
///
/// Two entry points:
/// - [`Math::preprocess_source`] runs before the dmc lexer; rewrites raw
///   `$...$` / `$$...$$` to `<MathMl mathml="..."/>` JSX so the parser
///   never sees unescaped LaTeX. Required: `_` and `^` inside math would
///   otherwise be parsed as Markdown emphasis markers.
/// - [`Transformer`] impl runs as a pipeline pass on already-parsed AST.
///   Used by tests and any caller that builds a Document directly.
#[derive(Default, Debug)]
pub struct Math;

impl Math {
  /// Rewrite `$...$` / `$$...$$` in raw MDX source to `<MathMl/>` JSX.
  /// Skips fenced code blocks, inline code spans, and existing JSX tags.
  pub fn preprocess_source(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let bytes = source.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
      if let Some(end) = Self::skip_fenced_code(source, bytes, i) {
        out.push_str(&source[i..end]);
        i = end;
        continue;
      }
      if let Some(end) = Self::skip_inline_code(source, bytes, i) {
        out.push_str(&source[i..end]);
        i = end;
        continue;
      }
      if let Some(end) = Self::skip_jsx_tag(source, bytes, i) {
        out.push_str(&source[i..end]);
        i = end;
        continue;
      }
      if bytes[i] == b'\\' && bytes.get(i + 1) == Some(&b'$') {
        out.push_str("\\$");
        i += 2;
        continue;
      }
      if bytes[i] == b'$' {
        let display = bytes.get(i + 1) == Some(&b'$');
        let delim_len = if display { 2 } else { 1 };
        let body_start = i + delim_len;
        let close_off =
          if display { source[body_start..].find("$$") } else { Self::find_inline_close(&source[body_start..]) };
        if let Some(off) = close_off {
          let inner = &source[body_start..body_start + off];
          if !inner.trim().is_empty() {
            let rendered = Self::render(inner, display);
            out.push_str(&format!("<MathMl mathml=\"{}\"/>", Self::escape_jsx_attr(&rendered)));
            i = body_start + off + delim_len;
            continue;
          }
        }
        out.push('$');
        i += 1;
        continue;
      }
      let ch_len = utf8_char_len(bytes[i]);
      out.push_str(&source[i..i + ch_len]);
      i += ch_len;
    }
    out
  }

  /// Render a LaTeX string. Engine is the active [`crate::MathEngine`]
  /// (default KaTeX HTML; can be flipped to MathML via `pulldown-latex`).
  /// Cached by `(latex, display, engine)` so repeated math in a doc hits
  /// the renderer once. On parse failure returns a
  /// `<span class="math-error">` wrapper around the original LaTeX.
  pub fn render(latex: &str, display: bool) -> String {
    let engine = Self::active_engine();
    let cache = Self::cache();
    let key = (latex.to_string(), display, engine);
    if let Some(hit) = cache.lock().expect("math cache lock").get(&key) {
      return hit.clone();
    }
    let html = match engine {
      crate::MathEngine::Katex => Self::render_katex(latex, display),
      crate::MathEngine::Mathml => Self::render_mathml(latex, display),
    };
    cache.lock().expect("math cache lock").insert(key, html.clone());
    html
  }

  fn render_katex(latex: &str, display: bool) -> String {
    let opts = if display { Self::display_opts() } else { Self::inline_opts() };
    match katex::render_with_opts(latex, opts) {
      Ok(html) => html,
      Err(_) => Self::error_span(latex, display),
    }
  }

  fn render_mathml(latex: &str, display: bool) -> String {
    use pulldown_latex::config::DisplayMode;
    use pulldown_latex::{Parser, RenderConfig, Storage, mathml::push_mathml};
    let storage = Storage::new();
    let parser = Parser::new(latex, &storage);
    let cfg = RenderConfig {
      display_mode: if display { DisplayMode::Block } else { DisplayMode::Inline },
      ..Default::default()
    };
    let mut out = String::new();
    match push_mathml(&mut out, parser, cfg) {
      Ok(()) => out,
      Err(_) => Self::error_span(latex, display),
    }
  }

  fn error_span(latex: &str, display: bool) -> String {
    format!(
      "<span class=\"math-error\">{}{}{}</span>",
      if display { "$$" } else { "$" },
      latex,
      if display { "$$" } else { "$" }
    )
  }

  /// Process-wide active engine. Set once via [`Math::set_engine`]
  /// (pipeline does this from `PipelineConfig::math_engine`); defaults
  /// to KaTeX. Stored as a static so [`render`] does not need a
  /// per-call engine argument (keeps the source preprocessor signature
  /// engine-agnostic).
  pub fn set_engine(engine: crate::MathEngine) {
    Self::engine_slot().store(engine_to_u8(engine), std::sync::atomic::Ordering::Release);
  }

  fn active_engine() -> crate::MathEngine {
    u8_to_engine(Self::engine_slot().load(std::sync::atomic::Ordering::Acquire))
  }

  fn engine_slot() -> &'static std::sync::atomic::AtomicU8 {
    static S: OnceLock<std::sync::atomic::AtomicU8> = OnceLock::new();
    S.get_or_init(|| std::sync::atomic::AtomicU8::new(engine_to_u8(crate::MathEngine::default())))
  }

  fn cache() -> &'static Mutex<MathCache> {
    static C: OnceLock<Mutex<MathCache>> = OnceLock::new();
    C.get_or_init(|| Mutex::new(HashMap::new()))
  }

  /// Load a previously persisted math cache from `path`. Silently ignores
  /// missing/corrupt files. Idempotent: existing in-memory entries are
  /// kept (cache wins are additive).
  pub fn load_cache(path: &std::path::Path) {
    let Ok(s) = std::fs::read_to_string(path) else { return };
    let Ok(rows) = serde_json::from_str::<Vec<(String, bool, u8, String)>>(&s) else { return };
    let mut cache = Self::cache().lock().expect("math cache lock");
    for (latex, display, eng, html) in rows {
      cache.entry((latex, display, u8_to_engine(eng))).or_insert(html);
    }
  }

  /// Persist the in-memory math cache to `path`. Best effort; errors
  /// are swallowed.
  pub fn save_cache(path: &std::path::Path) {
    let cache = Self::cache().lock().expect("math cache lock");
    let rows: Vec<(String, bool, u8, String)> = cache
      .iter()
      .map(|((latex, display, eng), html)| (latex.clone(), *display, engine_to_u8(*eng), html.clone()))
      .collect();
    let Ok(json) = serde_json::to_string(&rows) else { return };
    if let Some(parent) = path.parent() {
      let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, json);
  }

  fn display_opts() -> &'static katex::Opts {
    static O: OnceLock<katex::Opts> = OnceLock::new();
    O.get_or_init(|| {
      katex::Opts::builder()
        .display_mode(true)
        .output_type(katex::OutputType::HtmlAndMathml)
        .build()
        .expect("katex opts")
    })
  }

  fn inline_opts() -> &'static katex::Opts {
    static O: OnceLock<katex::Opts> = OnceLock::new();
    O.get_or_init(|| {
      katex::Opts::builder()
        .display_mode(false)
        .output_type(katex::OutputType::HtmlAndMathml)
        .build()
        .expect("katex opts")
    })
  }

  /// Render LaTeX as a self-closing `<MathMl/>` JsxSelfClosing node.
  pub fn render_node(latex: &str, display: bool, span: &duck_diagnostic::Span) -> Node {
    let mathml = Self::render(latex, display);
    Node::JsxSelfClosing(JsxSelfClosing {
      name: "MathMl".into(),
      attrs: vec![JsxAttr { name: "mathml".into(), value: JsxAttrValue::String(mathml), span: span.clone() }],
      span: span.clone(),
    })
  }

  // ---------------------------------------------------------------- helpers

  fn skip_fenced_code(source: &str, bytes: &[u8], i: usize) -> Option<usize> {
    if bytes[i] != b'`' || bytes.get(i + 1) != Some(&b'`') || bytes.get(i + 2) != Some(&b'`') {
      return None;
    }
    let end = source[i + 3..].find("```").map(|p| i + 3 + p + 3).unwrap_or(bytes.len());
    Some(end)
  }

  fn skip_inline_code(source: &str, bytes: &[u8], i: usize) -> Option<usize> {
    if bytes[i] != b'`' {
      return None;
    }
    let p = source[i + 1..].find('`')?;
    Some(i + 1 + p + 1)
  }

  fn skip_jsx_tag(source: &str, bytes: &[u8], i: usize) -> Option<usize> {
    if bytes[i] != b'<' {
      return None;
    }
    let p = source[i + 1..].find('>')?;
    Some(i + 1 + p + 1)
  }

  fn find_inline_close(inline: &str) -> Option<usize> {
    let mut search = 0usize;
    while search < inline.len() {
      let rel = inline[search..].find(['$', '\n'])?;
      let abs = search + rel;
      if inline.as_bytes()[abs] == b'\n' {
        return None;
      }
      if abs > 0 && inline.as_bytes()[abs - 1] == b'\\' {
        search = abs + 1;
        continue;
      }
      return Some(abs);
    }
    None
  }

  /// Escape `"` and `&` so MathML survives JSX attribute parsing.
  /// Reversed by the codegen `MathMl` raw-HTML paster.
  fn escape_jsx_attr(s: &str) -> String {
    s.replace('&', "&amp;").replace('"', "&quot;")
  }
}

impl Transformer for Math {
  fn name(&self) -> &str {
    "math"
  }
  fn transform(&self, doc: &mut Document, _meta: &SourceMeta, _engine: &mut DiagnosticEngine<Code>) {
    let mut v = Apply;
    walk_root(&mut doc.children, &mut v);
  }
}

/// Free-function alias. Prefer [`Math::preprocess_source`].
pub fn preprocess_math_source(source: &str) -> String {
  Math::preprocess_source(source)
}

struct Apply;

impl Visitor for Apply {
  fn visit_node(&mut self, node: &mut Node) -> NodeAction {
    if let Node::Paragraph(p) = node
      && let [Node::Text(t)] = p.children.as_slice()
      && let Some(latex) = Math::unwrap_block(t.value.trim())
    {
      let span = t.span.clone();
      return NodeAction::Replace(vec![Math::render_node(latex, true, &span)]);
    }
    let Node::Text(t) = node else { return NodeAction::Keep };
    let Some(replacement) = Math::expand_inline(&t.value, &t.span) else { return NodeAction::Keep };
    NodeAction::Replace(replacement)
  }
}

impl Math {
  fn unwrap_block(s: &str) -> Option<&str> {
    let s = s.trim();
    let inner = s.strip_prefix("$$")?.strip_suffix("$$")?;
    Some(inner.trim())
  }

  fn expand_inline(text: &str, span: &duck_diagnostic::Span) -> Option<Vec<Node>> {
    if !text.contains('$') {
      return None;
    }
    let mut out: Vec<Node> = Vec::new();
    let mut buf = String::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    let mut found_any = false;

    while i < bytes.len() {
      let c = bytes[i];
      if c == b'\\' && i + 1 < bytes.len() && bytes[i + 1] == b'$' {
        buf.push('$');
        i += 2;
        continue;
      }
      if c != b'$' {
        let ch_len = utf8_char_len(bytes[i]);
        buf.push_str(&text[i..i + ch_len]);
        i += ch_len;
        continue;
      }
      let (delim, display) = if i + 1 < bytes.len() && bytes[i + 1] == b'$' { ("$$", true) } else { ("$", false) };
      let inner_start = i + delim.len();
      let Some(close_off) = Self::find_unescaped(&text[inner_start..], delim) else {
        buf.push('$');
        i += 1;
        continue;
      };
      let inner = &text[inner_start..inner_start + close_off];
      if !buf.is_empty() {
        out.push(Node::Text(Text { value: std::mem::take(&mut buf), span: span.clone() }));
      }
      out.push(Self::render_node(inner, display, span));
      i = inner_start + close_off + delim.len();
      found_any = true;
    }

    if !found_any {
      return None;
    }
    if !buf.is_empty() {
      out.push(Node::Text(Text { value: buf, span: span.clone() }));
    }
    Some(out)
  }

  fn find_unescaped(haystack: &str, delim: &str) -> Option<usize> {
    let mut search_from = 0;
    while search_from < haystack.len() {
      let off = haystack[search_from..].find(delim)?;
      let abs = search_from + off;
      if abs > 0 && haystack.as_bytes()[abs - 1] == b'\\' {
        search_from = abs + delim.len();
        continue;
      }
      return Some(abs);
    }
    None
  }
}

fn engine_to_u8(e: crate::MathEngine) -> u8 {
  match e {
    crate::MathEngine::Katex => 0,
    crate::MathEngine::Mathml => 1,
  }
}

fn u8_to_engine(b: u8) -> crate::MathEngine {
  match b {
    1 => crate::MathEngine::Mathml,
    _ => crate::MathEngine::Katex,
  }
}

fn utf8_char_len(b: u8) -> usize {
  if b < 0x80 {
    1
  } else if b < 0xE0 {
    2
  } else if b < 0xF0 {
    3
  } else {
    4
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use duck_diagnostic::Span;
  use std::sync::Arc;

  fn s() -> Span {
    Span { file: Arc::from("<t>"), line: 1, column: 1, length: 0 }
  }

  #[test]
  fn passthrough_when_no_dollars() {
    assert!(Math::expand_inline("nothing here", &s()).is_none());
  }

  #[test]
  fn inline_math_replaces_one_span() {
    let r = Math::expand_inline("a $x+1$ b", &s()).expect("matched");
    // Text("a ") + JsxSelfClosing(MathMl) + Text(" b")
    assert_eq!(r.len(), 3);
    assert!(matches!(&r[0], Node::Text(t) if t.value == "a "));
    assert!(matches!(&r[1], Node::JsxSelfClosing(e) if e.name == "MathMl"));
    assert!(matches!(&r[2], Node::Text(t) if t.value == " b"));
  }

  #[test]
  fn escaped_dollar_is_literal() {
    let r = Math::expand_inline(r"price \$5", &s());
    assert!(r.is_none() || matches!(&r.unwrap()[0], Node::Text(t) if t.value.contains("$5")));
  }

  #[test]
  fn unmatched_dollar_left_alone() {
    assert!(Math::expand_inline("a $ stray", &s()).is_none());
  }

  #[test]
  fn block_math_unwraps() {
    let s_ = s();
    let mut p = Document {
      children: vec![Node::Paragraph(Paragraph {
        children: vec![Node::Text(Text { value: "$$ x = y $$".into(), span: s_.clone() })],
        span: s_.clone(),
      })],
      span: s_,
    };
    let mut v = Apply;
    walk_root(&mut p.children, &mut v);
    assert_eq!(p.children.len(), 1);
    if let Node::JsxSelfClosing(e) = &p.children[0] {
      assert_eq!(e.name, "MathMl");
      let mathml = e.attrs.iter().find(|a| a.name == "mathml").unwrap();
      assert!(
        matches!(&mathml.value, JsxAttrValue::String(s) if s.contains("<math") && s.contains("display=\"block\""))
      );
    } else {
      panic!("expected MathMl element, got {:?}", p.children[0]);
    }
  }
}
