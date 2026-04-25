use crate::pipeline::Transformer;
use duck_md_parser::ast::*;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, ThemeSet};
use syntect::html::{IncludeBackground, styled_line_to_highlighted_html};
use syntect::parsing::SyntaxSet;

pub struct PrettyCode {
  syntax_set: SyntaxSet,
  theme: syntect::highlighting::Theme,
  theme_name: String,
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
    Self { syntax_set, theme, theme_name: theme_name.to_string() }
  }

  fn highlight(&self, lang: Option<&str>, code: &str) -> Option<String> {
    let syntax = lang
      .and_then(|l| self.syntax_set.find_syntax_by_token(l))
      .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());
    let mut h = HighlightLines::new(syntax, &self.theme);
    let mut out = String::new();
    out.push_str("<pre class=\"pretty-code\" data-theme=\"");
    out.push_str(&self.theme_name);
    out.push('"');
    if let Some(l) = lang {
      out.push_str(" data-lang=\"");
      out.push_str(l);
      out.push('"');
    }
    out.push_str("><code>");
    for line in code.lines() {
      let regions: Vec<(Style, &str)> = h.highlight_line(line, &self.syntax_set).ok()?;
      let html_line = styled_line_to_highlighted_html(&regions, IncludeBackground::No).ok()?;
      out.push_str(&html_line);
      out.push('\n');
    }
    out.push_str("</code></pre>");
    Some(out)
  }
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
      cb.highlighted_html = self.pretty.highlight(cb.lang.as_deref(), &cb.value);
    }
    crate::visit::VisitFlow::Continue
  }
}
