use crate::{
  NodeSink, WalkCtx, Walker,
  escape::{escape_attr, escape_text, escape_url, sanitize_url},
};
use dmc_diagnostic::Code;
use dmc_parser::ast::*;
use duck_diagnostic::{DiagnosticEngine, diag};

#[derive(Debug, Clone, Copy, Default)]
pub struct RenderOptions {
  /// GFM disallowed raw HTML extension. When enabled, a fixed tag-name
  /// set gets its leading `<` escaped in raw HTML output.
  pub gfm_disallowed_raw_html: bool,
  /// Raw embedded HTML passthrough (CommonMark "unsafe" mode). When
  /// `false` (the default), raw HTML is NOT emitted verbatim: block-level
  /// raw HTML is omitted and inline raw HTML is escaped to visible text,
  /// so attacker-supplied `<script>` / `<iframe>` / `on*=` markup cannot
  /// reach the output. Set `true` to opt back into verbatim passthrough
  /// (the caller then owns the XSS risk).
  pub allow_dangerous_html: bool,
}

/// Static HTML emitter driven by walker enter/leave events. Tables are
/// rendered up-front on `enter Table` (rows/cells aren't `Node` variants)
/// and `in_table_depth` suppresses walker events on cell content.
pub struct HtmlEmitter {
  out: String,
  diag_engine: DiagnosticEngine<Code>,
  in_table_depth: usize,
  options: RenderOptions,
}

impl NodeSink for HtmlEmitter {
  fn enter(&mut self, node: &Node, ctx: &WalkCtx) {
    if self.in_table_depth > 0 {
      return;
    }
    self.maybe_separate_list_item_block_child(node, ctx);
    match node {
      Node::Text(t) => self.out.push_str(&escape_text(&t.value)),
      Node::InlineCode(c) => {
        self.out.push_str("<code>");
        self.out.push_str(&escape_text(&c.value));
        self.out.push_str("</code>");
      },
      Node::CodeBlock(cb) => self.code_block(cb),
      Node::Image(i) => self.image(i),
      Node::HorizontalRule(_) => self.out.push_str("<hr />\n"),
      Node::HardBreak(_) => self.out.push_str("<br />\n"),
      // Block-level raw HTML gets a trailing `\n` (CM line-per-block);
      // inline raw HTML inside a paragraph/heading must not.
      //
      // SEC-002: raw HTML passthrough is gated behind `allow_dangerous_html`.
      // When off (default, CommonMark "safe" mode): block-level raw HTML is
      // omitted entirely; inline raw HTML is escaped to visible text.
      Node::Html(h) => {
        let inline_context = matches!(ctx.parent, Some(Node::Paragraph(_)) | Some(Node::Heading(_)));
        if !self.options.allow_dangerous_html {
          if inline_context {
            self.out.push_str(&escape_text(&h.value));
          }
          // Block-level raw HTML: omitted entirely in safe mode.
          return;
        }
        let value =
          if self.options.gfm_disallowed_raw_html { escape_disallowed_raw_html_tag(&h.value) } else { h.value.clone() };
        self.out.push_str(&value);
        if !inline_context && !value.ends_with('\n') {
          self.out.push('\n');
        }
      },
      Node::SoftBreak(_) => self.out.push('\n'),
      Node::JsxSelfClosing(s) => self.jsx_self_closing(s),
      Node::JsxExpression(e) => {
        // Lower trivial string-literal expressions (`{' '}`, `{"x"}`,
        // `` {`y`} ``) to plain text; dynamic expressions still trip GW002.
        if let Some(text) = string_literal_expression(&e.value) {
          self.out.push_str(&escape_text(&text));
        } else {
          self.diag(Code::HtmlExpressionDropped, format!("html: raw `{{...}}` expression dropped: {}", e.value.trim()));
        }
      },
      Node::Table(t) => {
        self.in_table_depth += 1;
        self.inline_table(t);
      },
      Node::Frontmatter(_) | Node::Import(_) | Node::Export(_) => {},
      _ => self.open_tag(node),
    }
  }

  fn leave(&mut self, node: &Node, _ctx: &WalkCtx) {
    if let Node::Table(_) = node {
      self.in_table_depth = self.in_table_depth.saturating_sub(1);
      return;
    }
    if self.in_table_depth > 0 {
      return;
    }
    self.close_tag(node);
  }
}

impl Default for HtmlEmitter {
  fn default() -> Self {
    Self::new()
  }
}

impl HtmlEmitter {
  pub fn new() -> Self {
    Self::new_with_options(RenderOptions::default())
  }

  pub fn new_with_options(options: RenderOptions) -> Self {
    Self { out: String::new(), diag_engine: DiagnosticEngine::new(), in_table_depth: 0, options }
  }

  pub fn into_string(self) -> String {
    self.out
  }

  /// Returned `DiagnosticEngine` is per-emitter; merge into a shared
  /// engine via `outer.extend(diag)`.
  pub fn into_parts(self) -> (String, DiagnosticEngine<Code>) {
    (self.out, self.diag_engine)
  }

  /// Drive the walker; use when no other sink shares the walk.
  pub fn render(doc: &Document) -> (String, DiagnosticEngine<Code>) {
    let mut e = Self::new();
    Walker::new(doc).walk(&mut [&mut e]);
    e.into_parts()
  }

  pub fn render_with(doc: &Document, options: RenderOptions) -> (String, DiagnosticEngine<Code>) {
    let mut e = Self::new_with_options(options);
    Walker::new(doc).walk(&mut [&mut e]);
    e.into_parts()
  }

  fn diag(&mut self, code: Code, message: impl Into<String>) {
    self.diag_engine.emit(diag!(code, message.into()));
  }

  fn is_block_node(node: &Node) -> bool {
    matches!(
      node,
      Node::Paragraph(_)
        | Node::List(_)
        | Node::Blockquote(_)
        | Node::CodeBlock(_)
        | Node::Heading(_)
        | Node::HorizontalRule(_)
        | Node::Table(_)
        | Node::Html(_)
    )
  }

  fn maybe_separate_list_item_block_child(&mut self, node: &Node, ctx: &WalkCtx) {
    let Some(parent) = ctx.parent else {
      return;
    };
    if !matches!(parent, Node::ListItem(_) | Node::TaskListItem(_)) || ctx.index == 0 || !Self::is_block_node(node) {
      return;
    }
    let prev = Node::children_of(parent).get(ctx.index - 1);
    if prev.is_some_and(|n| !Self::is_block_node(n)) && !self.out.ends_with('\n') {
      self.out.push('\n');
    }
  }

  fn open_tag(&mut self, node: &Node) {
    match node {
      Node::Heading(h) => match &h.id {
        Some(id) => self.out.push_str(&format!("<h{} id=\"{}\">", h.level, escape_attr(id))),
        None => self.out.push_str(&format!("<h{}>", h.level)),
      },
      Node::Paragraph(_) => self.out.push_str("<p>"),
      Node::Bold(_) => self.out.push_str("<strong>"),
      Node::Italic(_) => self.out.push_str("<em>"),
      Node::Strikethrough(_) => self.out.push_str("<del>"),
      Node::Blockquote(_) => self.out.push_str("<blockquote>\n"),
      Node::List(l) => {
        let tag = if l.ordered { "ol" } else { "ul" };
        self.out.push('<');
        self.out.push_str(tag);
        // Match remark-gfm: parent gets `class="contains-task-list"`.
        if l.children.iter().any(|c| matches!(c, Node::TaskListItem(_))) {
          self.out.push_str(" class=\"contains-task-list\"");
        }
        if l.ordered
          && let Some(s) = l.start
          && s != 1
        {
          self.out.push_str(&format!(" start=\"{}\"", s));
        }
        self.out.push_str(">\n");
      },
      // CM: `<li>\n` for items with block children; tight items hug
      // inline content.
      Node::ListItem(li) => {
        let has_block_child = li.children.first().is_some_and(|c| {
          matches!(
            c,
            Node::Paragraph(_)
              | Node::List(_)
              | Node::Blockquote(_)
              | Node::CodeBlock(_)
              | Node::Heading(_)
              | Node::HorizontalRule(_)
              | Node::Table(_)
              | Node::Html(_)
          )
        });
        if has_block_child {
          self.out.push_str("<li>\n");
        } else {
          self.out.push_str("<li>");
        }
      },
      Node::TaskListItem(t) => {
        // remark-gfm shape: `<input type="checkbox" ...>` (no `/>`) plus
        // a literal trailing space before item content.
        let checked = if t.checked { " checked" } else { "" };
        self.out.push_str(&format!("<li class=\"task-list-item\"><input type=\"checkbox\"{} disabled> ", checked));
      },
      Node::Link(l) => {
        self.out.push_str(&format!("<a href=\"{}\"", escape_attr(&escape_url(sanitize_url(&l.href)))));
        // CM 6.3 / 4.7: link title -> anchor `title` attribute.
        // (autolink-headings tooltip borrows this same field.)
        if let Some(title) = &l.title {
          self.out.push_str(&format!(" title=\"{}\"", escape_attr(title)));
        }
        self.out.push('>');
      },
      Node::JsxElement(e) => {
        if e.name.is_empty() {
          self.diag(Code::MalformedJsxTagName, "html: JSX element has empty name; skipped".to_string());
          return;
        }
        // GFM Disallowed Raw HTML: escape `<` on the fixed tag-name set.
        if self.options.gfm_disallowed_raw_html && is_disallowed_raw_html(&e.name) {
          self.out.push_str("&lt;");
        } else {
          self.out.push('<');
        }
        self.out.push_str(&e.name);
        for a in &e.attrs {
          self.jsx_attr(a);
        }
        self.out.push('>');
      },
      Node::JsxFragment(_) => {},
      _ => {},
    }
  }

  /// Block-level closes get a trailing `\n` to match CM's line-per-block
  /// layout.
  fn close_tag(&mut self, node: &Node) {
    match node {
      Node::Heading(h) => self.out.push_str(&format!("</h{}>\n", h.level)),
      Node::Paragraph(_) => self.out.push_str("</p>\n"),
      Node::Bold(_) => self.out.push_str("</strong>"),
      Node::Italic(_) => self.out.push_str("</em>"),
      Node::Strikethrough(_) => self.out.push_str("</del>"),
      Node::Blockquote(_) => self.out.push_str("</blockquote>\n"),
      Node::List(l) => {
        let tag = if l.ordered { "ol" } else { "ul" };
        self.out.push_str(&format!("</{}>\n", tag));
      },
      Node::ListItem(_) | Node::TaskListItem(_) => self.out.push_str("</li>\n"),
      Node::Link(_) => self.out.push_str("</a>"),
      Node::JsxElement(e) if !e.name.is_empty() => {
        if self.options.gfm_disallowed_raw_html && is_disallowed_raw_html(&e.name) {
          self.out.push_str(&format!("&lt;/{}>", e.name));
        } else {
          self.out.push_str(&format!("</{}>", e.name));
        }
      },
      Node::JsxFragment(_) => {},
      _ => {},
    }
  }

  fn code_block(&mut self, cb: &CodeBlock) {
    self.out.push_str("<pre><code");
    if let Some(lang) = &cb.lang {
      self.out.push_str(&format!(" class=\"language-{}\"", escape_attr(lang)));
    }
    self.out.push('>');
    self.out.push_str(&escape_text(&cb.value));
    self.out.push_str("</code></pre>\n");
  }

  fn image(&mut self, i: &Image) {
    self.out.push_str(&format!(
      "<img src=\"{}\" alt=\"{}\"",
      escape_attr(&escape_url(sanitize_url(&i.src))),
      escape_attr(&i.alt)
    ));
    if let Some(title) = &i.title {
      self.out.push_str(&format!(" title=\"{}\"", escape_attr(title)));
    }
    // CM reference uses XHTML self-closing on `<img>`.
    self.out.push_str(" />");
  }

  fn jsx_self_closing(&mut self, s: &JsxSelfClosing) {
    if s.name.is_empty() {
      self.diag(Code::MalformedJsxTagName, "html: self-closing JSX has empty name; skipped".to_string());
      return;
    }
    match s.name.as_str() {
      // SEC-002: `MermaidSvg` / `MathMl` emit renderer-produced markup
      // (an `<svg>` / `<math>` document) verbatim. Both are derived from
      // attacker-influenced source (chart text / math source), so the
      // raw passthrough is gated behind `allow_dangerous_html`. In safe
      // mode the markup is dropped rather than escaped — an escaped SVG
      // document is meaningless as visible text.
      "MermaidSvg" => {
        if self.options.allow_dangerous_html
          && let Some(attr) = s.attrs.iter().find(|a| a.name == "svg")
          && let JsxAttrValue::String(svg) = &attr.value
        {
          self.out.push_str(svg);
        }
      },
      "MathMl" => {
        if self.options.allow_dangerous_html
          && let Some(attr) = s.attrs.iter().find(|a| a.name == "mathml")
          && let JsxAttrValue::String(mathml) = &attr.value
        {
          // Reverse the JSX-attr escape from Math::preprocess_source.
          let unescaped = mathml.replace("&quot;", "\"").replace("&amp;", "&");
          self.out.push_str(&unescaped);
        }
      },
      "PackageManagerTabs" => {
        self.out.push_str("<div class=\"gentledmc-pm-tabs\">");
        for pm in ["npm", "yarn", "pnpm", "bun"] {
          if let Some(attr) = s.attrs.iter().find(|a| a.name == pm)
            && let JsxAttrValue::String(cmd) = &attr.value
          {
            self.out.push_str(&format!(
              "<pre><code class=\"gentledmc-language-bash\" data-pm=\"{}\">{}</code></pre>",
              pm,
              escape_text(cmd)
            ));
          }
        }
        self.out.push_str("</div>");
      },
      _ => {
        self.out.push('<');
        self.out.push_str(&s.name);
        for a in &s.attrs {
          self.jsx_attr(a);
        }
        self.out.push_str(" />");
      },
    }
  }

  fn jsx_attr(&mut self, a: &JsxAttr) {
    self.out.push(' ');
    self.out.push_str(&a.name);
    match &a.value {
      // Match rehype/shiki: boolean JSX attrs serialize as empty-string
      // (`attr=""`) so consumer selectors keying off `[attr=""]` work.
      JsxAttrValue::Boolean => self.out.push_str("=\"\""),
      JsxAttrValue::String(s) => self.out.push_str(&format!("=\"{}\"", escape_attr(s))),
      JsxAttrValue::Expression(e) => self.out.push_str(&format!("={{{}}}", e)),
      // Spread has no HTML form; drop, and pop the leading space.
      JsxAttrValue::Spread(_) => {
        self.out.pop();
      },
    }
  }

  /// Render the entire `<table>...</table>` up-front; cell content uses
  /// `inline_node` recursion since the walker is suppressed inside.
  fn inline_table(&mut self, t: &Table) {
    self.out.push_str("<table>\n");
    if let Some(header) = t.children.first() {
      self.out.push_str("<thead>\n<tr>\n");
      for (i, cell) in header.cells.iter().enumerate() {
        self.inline_cell("th", cell, t.align.get(i).copied().unwrap_or(TableAlign::None));
      }
      self.out.push_str("</tr>\n</thead>\n");
    }
    if t.children.len() > 1 {
      self.out.push_str("<tbody>\n");
      for row in &t.children[1..] {
        self.out.push_str("<tr>\n");
        for (i, cell) in row.cells.iter().enumerate() {
          self.inline_cell("td", cell, t.align.get(i).copied().unwrap_or(TableAlign::None));
        }
        self.out.push_str("</tr>\n");
      }
      self.out.push_str("</tbody>\n");
    }
    self.out.push_str("</table>\n");
  }

  fn inline_cell(&mut self, tag: &str, cell: &TableCell, align: TableAlign) {
    self.out.push('<');
    self.out.push_str(tag);
    let align_str = match align {
      TableAlign::Left => Some("left"),
      TableAlign::Right => Some("right"),
      TableAlign::Center => Some("center"),
      TableAlign::None => None,
    };
    if let Some(a) = align_str {
      self.out.push_str(&format!(" align=\"{}\"", a));
    }
    self.out.push('>');
    for c in &cell.children {
      self.inline_node(c);
    }
    self.out.push_str("</");
    self.out.push_str(tag);
    self.out.push_str(">\n");
  }

  /// Self-recursive render for the table inline path (walker is
  /// suppressed via `in_table_depth`).
  fn inline_node(&mut self, node: &Node) {
    match node {
      Node::Text(t) => self.out.push_str(&escape_text(&t.value)),
      Node::Bold(i) => self.wrap_tag("strong", &i.children),
      Node::Italic(i) => self.wrap_tag("em", &i.children),
      Node::Strikethrough(i) => self.wrap_tag("del", &i.children),
      Node::InlineCode(c) => {
        self.out.push_str("<code>");
        self.out.push_str(&escape_text(&c.value));
        self.out.push_str("</code>");
      },
      Node::Link(l) => {
        self.out.push_str(&format!("<a href=\"{}\"", escape_attr(&escape_url(sanitize_url(&l.href)))));
        if let Some(label) = &l.title {
          self.out.push_str(&format!(" aria-label=\"{}\"", escape_attr(label)));
        }
        self.out.push('>');
        for c in &l.children {
          self.inline_node(c);
        }
        self.out.push_str("</a>");
      },
      Node::Image(i) => self.image(i),
      Node::HardBreak(_) => self.out.push_str("<br />\n"),
      Node::SoftBreak(_) => self.out.push('\n'),
      Node::CodeBlock(cb) => self.code_block(cb),
      _ => {
        self.open_tag(node);
        for kid in Node::children_of(node) {
          self.inline_node(kid);
        }
        self.close_tag(node);
      },
    }
  }

  fn wrap_tag(&mut self, tag: &str, children: &[Node]) {
    self.out.push('<');
    self.out.push_str(tag);
    self.out.push('>');
    for c in children {
      self.inline_node(c);
    }
    self.out.push_str("</");
    self.out.push_str(tag);
    self.out.push('>');
  }
}

/// GFM Disallowed Raw HTML tag set. ASCII case-insensitive.
fn is_disallowed_raw_html(name: &str) -> bool {
  matches!(
    name.to_ascii_lowercase().as_str(),
    "title" | "textarea" | "style" | "xmp" | "iframe" | "noembed" | "noframes" | "script" | "plaintext"
  )
}

fn escape_disallowed_raw_html_tag(raw: &str) -> String {
  let bytes = raw.as_bytes();
  let mut out = String::with_capacity(raw.len());
  let mut i = 0;
  while i < bytes.len() {
    if bytes[i] == b'<' {
      let mut j = i + 1;
      if j < bytes.len() && bytes[j] == b'/' {
        j += 1;
      }
      let name_start = j;
      while j < bytes.len() && ((bytes[j] as char).is_ascii_alphanumeric() || bytes[j] == b'-') {
        j += 1;
      }
      if j > name_start && is_disallowed_raw_html(&raw[name_start..j]) {
        out.push_str("&lt;");
        i += 1;
        continue;
      }
    }
    out.push(bytes[i] as char);
    i += 1;
  }
  out
}

pub fn render_html(doc: &Document) -> String {
  let mut e = HtmlEmitter::new();
  Walker::new(doc).walk(&mut [&mut e]);
  e.into_string()
}

pub fn render_html_with(doc: &Document, options: RenderOptions) -> String {
  let mut e = HtmlEmitter::new_with_options(options);
  Walker::new(doc).walk(&mut [&mut e]);
  e.into_string()
}

/// Match a JSX expression whose entire body is a single string literal
/// (single/double-quoted, or backtick template without `${...}`). Used
/// to lower idiomatic `{' '}` / `{"x"}` to plain text; dynamic
/// expressions return `None` and still trip GW002.
fn string_literal_expression(raw: &str) -> Option<String> {
  let s = raw.trim();
  if s.len() < 2 {
    return None;
  }
  let bytes = s.as_bytes();
  let q = bytes[0];
  if !matches!(q, b'\'' | b'"' | b'`') || bytes[bytes.len() - 1] != q {
    return None;
  }
  let inner = &s[1..s.len() - 1];
  // Reject unescaped `${` in templates - those need JS to evaluate.
  if q == b'`' {
    let mut prev_backslash = false;
    let bs = inner.as_bytes();
    let mut i = 0;
    while i + 1 < bs.len() {
      if !prev_backslash && bs[i] == b'$' && bs[i + 1] == b'{' {
        return None;
      }
      prev_backslash = bs[i] == b'\\' && !prev_backslash;
      i += 1;
    }
  }
  // Decode common JS escapes; unknown ones pass through verbatim
  // (full ECMA-262 escape semantics not needed here).
  let mut out = String::with_capacity(inner.len());
  let mut chars = inner.chars();
  while let Some(c) = chars.next() {
    if c != '\\' {
      out.push(c);
      continue;
    }
    match chars.next() {
      Some('n') => out.push('\n'),
      Some('t') => out.push('\t'),
      Some('r') => out.push('\r'),
      Some('\\') => out.push('\\'),
      Some('\'') => out.push('\''),
      Some('"') => out.push('"'),
      Some('`') => out.push('`'),
      Some(other) => {
        out.push('\\');
        out.push(other);
      },
      None => out.push('\\'),
    }
  }
  Some(out)
}

#[cfg(test)]
mod tests {
  use super::string_literal_expression;

  #[test]
  fn recognises_simple_quoted_strings() {
    assert_eq!(string_literal_expression("' '"), Some(" ".into()));
    assert_eq!(string_literal_expression("\"x\""), Some("x".into()));
    assert_eq!(string_literal_expression("`y`"), Some("y".into()));
  }

  #[test]
  fn rejects_template_with_interpolation() {
    assert!(string_literal_expression("`hi ${name}`").is_none());
  }

  #[test]
  fn rejects_dynamic_expression() {
    assert!(string_literal_expression("count").is_none());
    assert!(string_literal_expression("foo()").is_none());
    assert!(string_literal_expression("a + b").is_none());
  }

  #[test]
  fn decodes_common_escapes() {
    assert_eq!(string_literal_expression("'\\n'"), Some("\n".into()));
    assert_eq!(string_literal_expression("'\\\\'"), Some("\\".into()));
  }
}
