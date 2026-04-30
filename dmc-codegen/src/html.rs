use crate::escape::{escape_attr, escape_text};
use dmc_diagnostic::Code;
use dmc_parser::ast::*;
use duck_diagnostic::{Diagnostic, DiagnosticEngine};

/// Stateful HTML emitter. Walks a `Document`, accumulates output into `out`,
/// and reports any non-fatal issues into a caller-provided
/// [`DiagnosticEngine`]. JSX elements pass through verbatim; raw
/// `JsxExpression` nodes can't be evaluated by HTML and surface as
/// [`Code::HtmlExpressionDropped`] warnings.
pub struct HtmlEmitter<'a> {
  out: String,
  engine: &'a mut DiagnosticEngine<Code>,
}

/// Convenience: render a whole document to HTML with a throwaway engine
/// (diagnostics discarded).
pub fn render_html(doc: &Document) -> String {
  let mut engine = DiagnosticEngine::new();
  HtmlEmitter::render(doc, &mut engine)
}

impl<'a> HtmlEmitter<'a> {
  /// Build an emitter, walk `doc`, return the buffered HTML. Diagnostics emit
  /// straight into `engine`.
  pub fn render(doc: &Document, engine: &'a mut DiagnosticEngine<Code>) -> String {
    let mut e = Self { out: String::new(), engine };
    for n in &doc.children {
      e.emit(n);
    }
    e.out
  }

  /// Push a diagnostic into the shared engine.
  fn diag(&mut self, code: Code, message: impl Into<String>) {
    self.engine.emit(Diagnostic::new(code, message.into()));
  }

  /// Dispatch one node to its variant-specific emitter.
  pub fn emit(&mut self, node: &Node) {
    match node {
      Node::Document(d) => {
        for c in &d.children {
          self.emit(c);
        }
      },
      Node::Frontmatter(_) => {},
      Node::Import(_) | Node::Export(_) => {},
      Node::Heading(h) => self.emit_heading(h),
      Node::Paragraph(p) => self.emit_paragraph(p),
      Node::Text(t) => self.out.push_str(&escape_text(&t.value)),
      Node::Bold(i) => self.wrap_inline("strong", &i.children),
      Node::Italic(i) => self.wrap_inline("em", &i.children),
      Node::Strikethrough(i) => self.wrap_inline("del", &i.children),
      Node::InlineCode(c) => {
        self.out.push_str("<code>");
        self.out.push_str(&escape_text(&c.value));
        self.out.push_str("</code>");
      },
      Node::CodeBlock(cb) => self.emit_code_block(cb),
      Node::Link(l) => self.emit_link(l),
      Node::Image(i) => self.emit_image(i),
      Node::HorizontalRule(_) => self.out.push_str("<hr />"),
      Node::Blockquote(b) => self.emit_blockquote(b),
      Node::List(l) => self.emit_list(l),
      Node::ListItem(li) => self.emit_list_item(li),
      Node::TaskListItem(t) => self.emit_task_list_item(t),
      Node::Table(t) => self.emit_table(t),
      Node::TableRow(_) | Node::TableCell(_) => {},
      Node::JsxElement(e) => self.emit_jsx_element(e),
      Node::JsxSelfClosing(s) => self.emit_jsx_self_closing(s),
      Node::JsxFragment(f) => {
        for c in &f.children {
          self.emit(c);
        }
      },
      Node::JsxExpression(e) => {
        self.diag(
          Code::HtmlExpressionDropped,
          format!("html: raw `{{...}}` expression dropped: {}", e.value.trim()),
        );
      },
      Node::HardBreak(_) => self.out.push_str("<br/>"),
      Node::SoftBreak(_) => self.out.push('\n'),
    }
  }

  fn wrap_inline(&mut self, tag: &str, children: &[Node]) {
    self.out.push('<');
    self.out.push_str(tag);
    self.out.push('>');
    for c in children {
      self.emit(c);
    }
    self.out.push_str("</");
    self.out.push_str(tag);
    self.out.push('>');
  }

  fn emit_heading(&mut self, h: &Heading) {
    self.out.push_str(&format!("<h{} id=\"{}\">", h.level, escape_attr(&h.slug())));
    for c in &h.children {
      self.emit(c);
    }
    self.out.push_str(&format!("</h{}>", h.level));
  }

  fn emit_paragraph(&mut self, p: &Paragraph) {
    self.out.push_str("<p>");
    for c in &p.children {
      self.emit(c);
    }
    self.out.push_str("</p>");
  }

  fn emit_code_block(&mut self, cb: &CodeBlock) {
    self.out.push_str("<pre><code");
    if let Some(lang) = &cb.lang {
      self.out.push_str(&format!(" class=\"gentledmc-language-{}\"", escape_attr(lang)));
    }
    self.out.push('>');
    self.out.push_str(&escape_text(&cb.value));
    self.out.push_str("</code></pre>");
  }

  fn emit_link(&mut self, l: &Link) {
    self.out.push_str(&format!("<a href=\"{}\"", escape_attr(&l.href)));
    if let Some(title) = &l.title {
      self.out.push_str(&format!(" aria-label=\"{}\"", escape_attr(title)));
    }
    self.out.push('>');
    for c in &l.children {
      self.emit(c);
    }
    self.out.push_str("</a>");
  }

  fn emit_image(&mut self, i: &Image) {
    self.out.push_str(&format!(
      "<img src=\"{}\" alt=\"{}\"",
      escape_attr(&i.src),
      escape_attr(&i.alt)
    ));
    if let Some(title) = &i.title {
      self.out.push_str(&format!(" title=\"{}\"", escape_attr(title)));
    }
    self.out.push_str(" />");
  }

  fn emit_blockquote(&mut self, b: &Blockquote) {
    self.out.push_str("<blockquote>");
    for c in &b.children {
      self.emit(c);
    }
    self.out.push_str("</blockquote>");
  }

  fn emit_list(&mut self, l: &List) {
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
    for c in &l.children {
      self.emit(c);
    }
    self.out.push_str("</");
    self.out.push_str(tag);
    self.out.push('>');
  }

  fn emit_list_item(&mut self, li: &ListItem) {
    self.out.push_str("<li>");
    for c in &li.children {
      self.emit(c);
    }
    self.out.push_str("</li>");
  }

  fn emit_table(&mut self, t: &Table) {
    self.out.push_str("<table>");
    if let Some(header) = t.children.first() {
      self.out.push_str("<thead><tr>");
      for (i, cell) in header.cells.iter().enumerate() {
        self.emit_cell("th", cell, t.align.get(i).copied().unwrap_or(TableAlign::None));
      }
      self.out.push_str("</tr></thead>");
    }
    if t.children.len() > 1 {
      self.out.push_str("<tbody>");
      for row in &t.children[1..] {
        self.out.push_str("<tr>");
        for (i, cell) in row.cells.iter().enumerate() {
          self.emit_cell("td", cell, t.align.get(i).copied().unwrap_or(TableAlign::None));
        }
        self.out.push_str("</tr>");
      }
      self.out.push_str("</tbody>");
    }
    self.out.push_str("</table>");
  }

  fn emit_cell(&mut self, tag: &str, cell: &TableCell, align: TableAlign) {
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
      self.emit(c);
    }
    self.out.push_str("</");
    self.out.push_str(tag);
    self.out.push('>');
  }

  fn emit_task_list_item(&mut self, t: &TaskListItem) {
    let checked = if t.checked { " checked" } else { "" };
    self.out.push_str(&format!("<li><input type=\"checkbox\" disabled{} />", checked));
    for c in &t.children {
      self.emit(c);
    }
    self.out.push_str("</li>");
  }

  fn emit_jsx_element(&mut self, e: &JsxElement) {
    if e.name.is_empty() {
      self.diag(Code::MalformedJsxTagName, "html: JSX element has empty name; skipped".to_string());
      return;
    }
    self.out.push('<');
    self.out.push_str(&e.name);
    for a in &e.attrs {
      self.emit_attr(a);
    }
    self.out.push('>');
    for c in &e.children {
      self.emit(c);
    }
    self.out.push_str("</");
    self.out.push_str(&e.name);
    self.out.push('>');
  }

  fn emit_jsx_self_closing(&mut self, s: &JsxSelfClosing) {
    if s.name.is_empty() {
      self.diag(
        Code::MalformedJsxTagName,
        "html: self-closing JSX has empty name; skipped".to_string(),
      );
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
          self.emit_attr(a);
        }
        self.out.push_str(" />");
      },
    }
  }

  fn emit_attr(&mut self, a: &JsxAttr) {
    self.out.push(' ');
    self.out.push_str(&a.name);
    match &a.value {
      JsxAttrValue::Boolean => {},
      JsxAttrValue::String(s) => {
        self.out.push_str(&format!("=\"{}\"", escape_attr(s)));
      },
      JsxAttrValue::Expression(e) => {
        self.out.push_str(&format!("={{{}}}", e));
      },
    }
  }
}
