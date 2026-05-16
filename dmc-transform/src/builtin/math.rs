//! LaTeX -> KaTeX/MathML. See `transformers/math.md` for full docs.

use crate::pipeline::Transformer;
use crate::visit::{NodeAction, Visitor, walk_root};
use dmc_diagnostic::metadata::SourceMeta;
use dmc_diagnostic::{Code, DiagResult};
use dmc_parser::ast::*;
use duck_diagnostic::{DiagnosticEngine, diag};
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
    if let Some(end) = Self::skip_frontmatter(source, bytes) {
      out.push_str(&source[..end]);
      i = end;
    }
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
    let opts_result = if display { Self::display_opts() } else { Self::inline_opts() };
    let opts = match opts_result {
      Ok(o) => o,
      // KaTeX builder failure -> fall back to the error placeholder
      // so the build still completes. The diagnostic itself is
      // discarded here because `render_katex` has no
      // `&mut DiagnosticEngine` handle; callers that need to capture
      // it should invoke `inline_opts()` / `display_opts()` directly
      // and propagate the `Diagnostic<Code>`.
      Err(_) => return Self::error_span(latex, display),
    };
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
  /// to KaTeX. Stored as a static so [`Self::render`] does not need a
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

  /// Load a previously persisted math cache from `path`. Missing or
  /// corrupt files yield `Ok(())` (empty cache). Other IO errors
  /// propagate as `IoRead`.
  #[allow(clippy::result_large_err)]
  pub fn load_cache(path: &std::path::Path) -> DiagResult {
    let s = match std::fs::read_to_string(path) {
      Ok(s) => s,
      Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
      Err(e) => return Err(diag!(Code::IoRead, format!("math cache read at {}: {}", path.display(), e))),
    };

    let rows = match serde_json::from_str::<Vec<(String, bool, u8, String)>>(&s) {
      Ok(r) => r,
      Err(_) => return Ok(()),
    };

    let mut cache = Self::cache().lock().map_err(|e| diag!(Code::LockPoisoned, format!("math cache lock: {}", e)))?;

    for (latex, display, eng, html) in rows {
      cache.entry((latex, display, u8_to_engine(eng))).or_insert(html);
    }
    Ok(())
  }

  /// Persist the in-memory math cache to `path`. Best effort; errors
  /// are swallowed.
  #[allow(clippy::result_large_err)]
  pub fn save_cache(path: &std::path::Path) -> DiagResult {
    let cache = Self::cache().lock().expect("math cache lock");
    let rows: Vec<(String, bool, u8, String)> = cache
      .iter()
      .map(|((latex, display, eng), html)| (latex.clone(), *display, engine_to_u8(*eng), html.clone()))
      .collect();

    let json =
      serde_json::to_string(&rows).map_err(|e| diag!(Code::JsonSerialize, format!("math cache serialise: {}", e)))?;

    if let Some(parent) = path.parent() {
      std::fs::create_dir_all(parent)
        .map_err(|e| diag!(Code::IoCreateDir, format!("math cache dir at {}: {}", parent.display(), e)))?;
    }

    std::fs::write(path, json)
      .map_err(|e| diag!(Code::IoWrite, format!("math cache write at {}: {}", path.display(), e)))?;
    Ok(())
  }

  #[allow(clippy::result_large_err)]
  fn display_opts() -> DiagResult<&'static katex::Opts> {
    static O: OnceLock<Result<katex::Opts, String>> = OnceLock::new();
    let cached = O.get_or_init(|| {
      katex::Opts::builder()
        .display_mode(true)
        .output_type(katex::OutputType::HtmlAndMathml)
        .build()
        .map_err(|e| e.to_string())
    });
    cached.as_ref().map_err(|e| diag!(Code::KatexOpts, format!("katex opts: {}", e)))
  }

  /// Build the KaTeX renderer once and cache. Inputs are all
  /// hard-coded constants, so any builder failure here is a packaging
  /// bug (e.g. a busted katex feature combo) - we surface it as
  /// `Code::KatexOpts` (warning, not fatal) and let the caller decide
  /// what to do with the unrenderable span.
  #[allow(clippy::result_large_err)]
  fn inline_opts() -> DiagResult<&'static katex::Opts> {
    static O: OnceLock<Result<katex::Opts, String>> = OnceLock::new();
    let cached = O.get_or_init(|| {
      katex::Opts::builder()
        .display_mode(false)
        .output_type(katex::OutputType::HtmlAndMathml)
        .build()
        .map_err(|e| e.to_string())
    });
    cached.as_ref().map_err(|e| diag!(Code::KatexOpts, format!("katex opts: {}", e)))
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

  // helpers

  /// Skip YAML (`---`) or TOML (`+++`) frontmatter at byte 0 so `$` runs
  /// inside `description`, etc. never get pair-matched as math spans.
  /// Returns the byte offset of the first content byte after the closing
  /// fence's newline, or `None` if no frontmatter is present.
  fn skip_frontmatter(source: &str, bytes: &[u8]) -> Option<usize> {
    let fence = if bytes.starts_with(b"---\n") || bytes.starts_with(b"---\r\n") {
      "---"
    } else if bytes.starts_with(b"+++\n") || bytes.starts_with(b"+++\r\n") {
      "+++"
    } else {
      return None;
    };
    let body_start = if bytes[3] == b'\r' { 5 } else { 4 };
    let rest = &source[body_start..];
    // Closing fence must sit at the start of a line. Scan for `\n<fence>`
    // and accept either bare-EOL or trailing-newline termination.
    let mut search = 0usize;
    while let Some(rel) = rest[search..].find(fence) {
      let abs = search + rel;
      let at_line_start = abs == 0 || rest.as_bytes()[abs - 1] == b'\n';
      let after = abs + fence.len();
      let terminates = after == rest.len() || rest.as_bytes()[after] == b'\n' || rest.as_bytes()[after] == b'\r';
      if at_line_start && terminates {
        let mut end = body_start + after;
        if end < bytes.len() && bytes[end] == b'\r' {
          end += 1;
        }
        if end < bytes.len() && bytes[end] == b'\n' {
          end += 1;
        }
        return Some(end);
      }
      search = abs + fence.len();
    }
    None
  }

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
