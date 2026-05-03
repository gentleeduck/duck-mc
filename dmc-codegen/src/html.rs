use crate::{
  NodeSink, WalkCtx, Walker,
  escape::{escape_attr, escape_text},
};
use dmc_diagnostic::Code;
use dmc_parser::ast::*;
use duck_diagnostic::{Diagnostic, DiagnosticEngine};

/// Emits static HTML by reacting to walker enter/leave events. Container
/// nodes split into `open_tag` / `close_tag` halves; leaves write their
/// markup once on enter. Tables are rendered up-front in `enter Table`
/// (rows + cells aren't `Node` variants the walker can surface) and
/// `in_table_depth` suppresses subsequent walker events on cell content.
///
/// Owns its own `DiagnosticEngine` during the walk; merge into the
/// caller's engine via `into_parts` after the walk completes.
pub struct HtmlEmitter {
  out: String,
  diag_engine: DiagnosticEngine<Code>,
  in_table_depth: usize,
}

impl NodeSink for HtmlEmitter {
  fn enter(&mut self, node: &Node, _ctx: &WalkCtx) {
    if self.in_table_depth > 0 {
      return;
    }
    match node {
      Node::Text(t) => self.out.push_str(&escape_text(&t.value)),
      Node::InlineCode(c) => {
        self.out.push_str("<code>");
        self.out.push_str(&escape_text(&c.value));
        self.out.push_str("</code>");
      },
      Node::CodeBlock(cb) => self.code_block(cb),
      Node::Image(i) => self.image(i),
      Node::HorizontalRule(_) => self.out.push_str("<hr />"),
      Node::HardBreak(_) => self.out.push_str("<br/>"),
      Node::SoftBreak(_) => self.out.push('\n'),
      Node::JsxSelfClosing(s) => self.jsx_self_closing(s),
      Node::JsxExpression(e) => {
        self.diag(Code::HtmlExpressionDropped, format!("html: raw `{{...}}` expression dropped: {}", e.value.trim()));
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
    Self { out: String::new(), diag_engine: DiagnosticEngine::new(), in_table_depth: 0 }
  }

  pub fn into_string(self) -> String {
    self.out
  }

  /// Take both buffers: the rendered HTML and the per-emitter diagnostic
  /// engine. Caller merges the diags into a shared engine via
  /// `outer.extend(diag)`.
  pub fn into_parts(self) -> (String, DiagnosticEngine<Code>) {
    (self.out, self.diag_engine)
  }

  /// Drive the walker; return `(html, diag)`. Use when no other sink
  /// shares the walk.
  pub fn render(doc: &Document) -> (String, DiagnosticEngine<Code>) {
    let mut e = Self::new();
    Walker::new(doc).walk(&mut [&mut e]);
    e.into_parts()
  }

  fn diag(&mut self, code: Code, message: impl Into<String>) {
    self.diag_engine.emit(Diagnostic::new(code, message.into()));
  }

  // --- container open / close (walker fills the children in between) ----

  /// Write the opening tag for a container node.
  fn open_tag(&mut self, node: &Node) {
    match node {
      Node::Heading(h) => self.out.push_str(&format!("<h{} id=\"{}\">", h.level, escape_attr(&h.slug()))),
      Node::Paragraph(_) => self.out.push_str("<p>"),
      Node::Bold(_) => self.out.push_str("<strong>"),
      Node::Italic(_) => self.out.push_str("<em>"),
      Node::Strikethrough(_) => self.out.push_str("<del>"),
      Node::Blockquote(_) => self.out.push_str("<blockquote>"),
      Node::List(l) => {
        let tag = if l.ordered { "ol" } else { "ul" };
        self.out.push('<');
        self.out.push_str(tag);
        if l.ordered
          && let Some(s) = l.start
          && s != 1
        {
          self.out.push_str(&format!(" start=\"{}\"", s));
        }
        self.out.push('>');
      },
      Node::ListItem(_) => self.out.push_str("<li>"),
      Node::TaskListItem(t) => {
        let checked = if t.checked { " checked" } else { "" };
        self.out.push_str(&format!("<li><input type=\"checkbox\" disabled{} />", checked));
      },
      Node::Link(l) => {
        self.out.push_str(&format!("<a href=\"{}\"", escape_attr(&l.href)));
        if let Some(title) = &l.title {
          self.out.push_str(&format!(" aria-label=\"{}\"", escape_attr(title)));
        }
        self.out.push('>');
      },
      Node::JsxElement(e) => {
        if e.name.is_empty() {
          self.diag(Code::MalformedJsxTagName, "html: JSX element has empty name; skipped".to_string());
          return;
        }
        self.out.push('<');
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

  /// Write the closing tag for a container node opened by `open_tag`.
  fn close_tag(&mut self, node: &Node) {
    match node {
      Node::Heading(h) => self.out.push_str(&format!("</h{}>", h.level)),
      Node::Paragraph(_) => self.out.push_str("</p>"),
      Node::Bold(_) => self.out.push_str("</strong>"),
      Node::Italic(_) => self.out.push_str("</em>"),
      Node::Strikethrough(_) => self.out.push_str("</del>"),
      Node::Blockquote(_) => self.out.push_str("</blockquote>"),
      Node::List(l) => {
        let tag = if l.ordered { "ol" } else { "ul" };
        self.out.push_str(&format!("</{}>", tag));
      },
      Node::ListItem(_) | Node::TaskListItem(_) => self.out.push_str("</li>"),
      Node::Link(_) => self.out.push_str("</a>"),
      Node::JsxElement(e) => {
        if !e.name.is_empty() {
          self.out.push_str(&format!("</{}>", e.name));
        }
      },
      Node::JsxFragment(_) => {},
      _ => {},
    }
  }

  // --- leaf-shaped emitters --------------------------------------------

  fn code_block(&mut self, cb: &CodeBlock) {
    self.out.push_str("<pre><code");
    if let Some(lang) = &cb.lang {
      self.out.push_str(&format!(" class=\"gentledmc-language-{}\"", escape_attr(lang)));
    }
    self.out.push('>');
    self.out.push_str(&escape_text(&cb.value));
    self.out.push_str("</code></pre>");
  }

  fn image(&mut self, i: &Image) {
    self.out.push_str(&format!("<img src=\"{}\" alt=\"{}\"", escape_attr(&i.src), escape_attr(&i.alt)));
    if let Some(title) = &i.title {
      self.out.push_str(&format!(" title=\"{}\"", escape_attr(title)));
    }
    self.out.push_str(" />");
  }

  fn jsx_self_closing(&mut self, s: &JsxSelfClosing) {
    if s.name.is_empty() {
      self.diag(Code::MalformedJsxTagName, "html: self-closing JSX has empty name; skipped".to_string());
      return;
    }
    match s.name.as_str() {
      "MermaidSvg" => {
        if let Some(attr) = s.attrs.iter().find(|a| a.name == "svg")
          && let JsxAttrValue::String(svg) = &attr.value
        {
          self.out.push_str(svg);
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
      JsxAttrValue::Boolean => {},
      JsxAttrValue::String(s) => self.out.push_str(&format!("=\"{}\"", escape_attr(s))),
      JsxAttrValue::Expression(e) => self.out.push_str(&format!("={{{}}}", e)),
    }
  }

  // --- table inline path (walker can't surface row/cell events) --------

  /// Render the entire `<table>...</table>` up-front. Cell content uses
  /// `inline_node` recursion since the walker is suppressed inside.
  fn inline_table(&mut self, t: &Table) {
    self.out.push_str("<table>");
    if let Some(header) = t.children.first() {
      self.out.push_str("<thead><tr>");
      for (i, cell) in header.cells.iter().enumerate() {
        self.inline_cell("th", cell, t.align.get(i).copied().unwrap_or(TableAlign::None));
      }
      self.out.push_str("</tr></thead>");
    }
    if t.children.len() > 1 {
      self.out.push_str("<tbody>");
      for row in &t.children[1..] {
        self.out.push_str("<tr>");
        for (i, cell) in row.cells.iter().enumerate() {
          self.inline_cell("td", cell, t.align.get(i).copied().unwrap_or(TableAlign::None));
        }
        self.out.push_str("</tr>");
      }
      self.out.push_str("</tbody>");
    }
    self.out.push_str("</table>");
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
    self.out.push('>');
  }

  /// Self-recursive render used only inside the table inline path. The
  /// walker is suppressed via `in_table_depth`, so cell content doesn't
  /// get a second pass.
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
        self.out.push_str(&format!("<a href=\"{}\"", escape_attr(&l.href)));
        if let Some(title) = &l.title {
          self.out.push_str(&format!(" aria-label=\"{}\"", escape_attr(title)));
        }
        self.out.push('>');
        for c in &l.children {
          self.inline_node(c);
        }
        self.out.push_str("</a>");
      },
      Node::Image(i) => self.image(i),
      Node::HardBreak(_) => self.out.push_str("<br/>"),
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

/// Convenience: render `doc` to HTML with a throwaway diagnostic engine.
pub fn render_html(doc: &Document) -> String {
  let mut e = HtmlEmitter::new();
  Walker::new(doc).walk(&mut [&mut e]);
  e.into_string()
}
