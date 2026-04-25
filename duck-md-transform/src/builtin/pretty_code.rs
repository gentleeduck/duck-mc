use crate::pipeline::Transformer;
use duck_md_parser::ast::*;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, ThemeSet};
use syntect::html::{IncludeBackground, styled_line_to_highlighted_html};
use syntect::parsing::SyntaxSet;

pub struct PrettyCode {
  syntax_set: SyntaxSet,
  light_theme: syntect::highlighting::Theme,
  light_theme_name: String,
  dark_theme: Option<syntect::highlighting::Theme>,
  dark_theme_name: Option<String>,
}

impl Default for PrettyCode {
  fn default() -> Self {
    Self::new("base16-ocean.dark")
  }
}

impl PrettyCode {
  pub fn new(theme_name: &str) -> Self {
    let syntax_set = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    let theme =
      ts.themes.get(theme_name).cloned().unwrap_or_else(|| ts.themes["base16-ocean.dark"].clone());
    Self {
      syntax_set,
      light_theme: theme,
      light_theme_name: theme_name.to_string(),
      dark_theme: None,
      dark_theme_name: None,
    }
  }

  pub fn dual(light_name: &str, dark_name: &str) -> Self {
    let syntax_set = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    let pick = |n: &str| ts.themes.get(n).cloned()
      .unwrap_or_else(|| ts.themes["base16-ocean.dark"].clone());
    Self {
      syntax_set,
      light_theme: pick(light_name),
      light_theme_name: light_name.to_string(),
      dark_theme: Some(pick(dark_name)),
      dark_theme_name: Some(dark_name.to_string()),
    }
  }

  fn highlight(
    &self,
    lang: Option<&str>,
    code: &str,
    marks: &[(usize, usize)],
    word_marks: &[String],
  ) -> Option<String> {
    let light = self.render_one(lang, code, marks, word_marks, &self.light_theme, &self.light_theme_name, "light")?;
    if let (Some(t), Some(name)) = (&self.dark_theme, &self.dark_theme_name) {
      let dark = self.render_one(lang, code, marks, word_marks, t, name, "dark")?;
      Some(format!(
        "<div data-rehype-pretty-code-fragment>{light}{dark}</div>"
      ))
    } else {
      Some(light)
    }
  }

  fn render_one(
    &self,
    lang: Option<&str>,
    code: &str,
    marks: &[(usize, usize)],
    word_marks: &[String],
    theme: &syntect::highlighting::Theme,
    theme_name: &str,
    appearance: &str,
  ) -> Option<String> {
    let syntax = lang
      .and_then(|l| self.syntax_set.find_syntax_by_token(l))
      .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());
    let mut h = HighlightLines::new(syntax, theme);
    let mut out = String::new();
    out.push_str("<pre class=\"pretty-code\" data-theme=\"");
    out.push_str(theme_name);
    out.push_str("\" data-appearance=\"");
    out.push_str(appearance);
    out.push('"');
    if let Some(l) = lang {
      out.push_str(" data-lang=\"");
      out.push_str(l);
      out.push('"');
    }
    out.push_str("><code>");
    for (idx, line) in code.lines().enumerate() {
      let line_no = idx + 1;
      let highlighted = marks.iter().any(|(a, b)| line_no >= *a && line_no <= *b);
      out.push_str(if highlighted {
        "<span class=\"line line--highlighted\">"
      } else {
        "<span class=\"line\">"
      });
      let regions: Vec<(Style, &str)> = h.highlight_line(line, &self.syntax_set).ok()?;
      let html_line = styled_line_to_highlighted_html(&regions, IncludeBackground::No).ok()?;
      let with_words = apply_word_marks(&html_line, word_marks);
      out.push_str(&with_words);
      out.push_str("</span>\n");
    }
    out.push_str("</code></pre>");
    Some(out)
  }
}

fn apply_word_marks(html: &str, words: &[String]) -> String {
  if words.is_empty() { return html.to_string(); }
  let mut s = html.to_string();
  for w in words {
    if w.is_empty() { continue; }
    let needle = w;
    let replacement = format!("<span class=\"word--highlighted\">{needle}</span>");
    s = s.replace(needle, &replacement);
  }
  s
}

fn parse_word_marks_meta(meta: Option<&str>) -> Vec<String> {
  let Some(s) = meta else { return Vec::new() };
  let mut out = Vec::new();
  let mut chars = s.chars().peekable();
  let mut buf = String::new();
  let mut inside = false;
  while let Some(c) = chars.next() {
    if c == '/' {
      if inside {
        if !buf.is_empty() { out.push(std::mem::take(&mut buf)); }
        inside = false;
      } else if matches!(chars.peek(), Some(p) if !p.is_whitespace()) {
        inside = true;
      }
    } else if inside {
      buf.push(c);
    }
  }
  out
}

fn parse_marks_meta(meta: Option<&str>) -> Vec<(usize, usize)> {
  let Some(s) = meta else { return Vec::new() };
  let mut out = Vec::new();
  let mut depth = 0i32;
  let mut buf = String::new();
  let mut in_braces = false;
  for ch in s.chars() {
    match ch {
      '{' if depth == 0 => { in_braces = true; depth += 1; }
      '{' => { depth += 1; buf.push(ch); }
      '}' if depth == 1 => {
        for token in buf.split(',') {
          let token = token.trim();
          if let Some((a, b)) = token.split_once('-') {
            if let (Ok(a), Ok(b)) = (a.trim().parse::<usize>(), b.trim().parse::<usize>()) {
              if a >= 1 && b >= a { out.push((a, b)); }
            }
          } else if let Ok(n) = token.parse::<usize>() {
            if n >= 1 { out.push((n, n)); }
          }
        }
        depth = 0;
        in_braces = false;
        buf.clear();
      }
      '}' => { depth -= 1; buf.push(ch); }
      c if in_braces => buf.push(c),
      _ => {}
    }
  }
  out
}

impl Transformer for PrettyCode {
  fn name(&self) -> &str {
    "pretty-code"
  }

  fn transform(&self, doc: &mut Document) {
    let mut walker = Walker { pretty: self };
    for c in &mut doc.children {
      crate::visit::walk_mut(c, &mut walker);
    }
  }
}

struct Walker<'a> {
  pretty: &'a PrettyCode,
}

impl<'a> crate::visit::Visitor for Walker<'a> {
  fn visit_node(&mut self, node: &mut Node) -> crate::visit::VisitFlow {
    if let Node::CodeBlock(cb) = node
      && cb.highlighted_html.is_none()
    {
      let marks = parse_marks_meta(cb.meta.as_deref());
      let words = parse_word_marks_meta(cb.meta.as_deref());
      cb.highlighted_html = self.pretty.highlight(cb.lang.as_deref(), &cb.value, &marks, &words);
    }
    crate::visit::VisitFlow::Continue
  }
}
