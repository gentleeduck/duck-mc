use duck_diagnostic::{Diagnostic, DiagnosticEngine};
use duck_md_diagnostic::Code;
use duck_md_parser::ast::*;

/// Emits an MDX-runtime JS body — a `_createMdxContent(props)` function that
/// returns a React tree built from `jsx`, `jsxs`, and `Fragment`. Imports +
/// exports are hoisted to the prelude; frontmatter is treated as sidecar data.
///
/// Unsupported nodes (currently only GFM `Table`) emit a
/// [`Code::MdxTableUnsupported`] warning rather than failing.
pub struct MdxBodyEmitter<'a> {
  out: String,
  imports: Vec<String>,
  exports: Vec<String>,
  engine: &'a mut DiagnosticEngine<Code>,
}

/// Convenience: render a whole document to an MDX body string with a
/// throwaway engine (diagnostics discarded).
pub fn render_mdx_body(doc: &Document) -> String {
  let mut engine = DiagnosticEngine::new();
  MdxBodyEmitter::render(doc, &mut engine)
}

impl<'a> MdxBodyEmitter<'a> {
  /// Build an emitter, walk `doc`, return the body. Diagnostics emit
  /// straight into `engine`.
  pub fn render(doc: &Document, engine: &'a mut DiagnosticEngine<Code>) -> String {
    let mut e = Self {
      out: String::new(),
      imports: Vec::new(),
      exports: Vec::new(),
      engine,
    };
    e.emit_document(doc);
    e.finish()
  }

  fn diag(&mut self, code: Code, message: impl Into<String>) {
    self.engine.emit(Diagnostic::new(code, message.into()));
  }

  /// Wrap the emitted body in the `_createMdxContent` runtime shell.
  fn finish(self) -> String {
    let mut prelude = String::new();
    for i in &self.imports {
      prelude.push_str(i);
      prelude.push('\n');
    }
    for e in &self.exports {
      prelude.push_str(e);
      prelude.push('\n');
    }
    format!(
      "{prelude}function _createMdxContent(props) {{\n  const _components = (props && props.components) || {{}};\n  const {{ Fragment, jsx, jsxs }} = arguments[0];\n  return {body};\n}}\nreturn _createMdxContent(arguments[0]);\n",
      prelude = prelude,
      body = self.out,
    )
  }

  /// Hoist top-level imports/exports, drop frontmatter, render rest as a
  /// single `Fragment` children array.
  pub fn emit_document(&mut self, doc: &Document) {
    let mut content_children: Vec<&Node> = Vec::new();
    for n in &doc.children {
      match n {
        Node::Import(i) => self.imports.push(i.raw.trim_end().to_string()),
        Node::Export(x) => self.exports.push(x.raw.trim_end().to_string()),
        Node::Frontmatter(_) => {},
        _ => content_children.push(n),
      }
    }
    let body = self.emit_children_array(&content_children);
    self.out = format!("jsxs(Fragment, {{ children: {} }})", body);
  }

  fn emit_children_array(&mut self, nodes: &[&Node]) -> String {
    let mut parts: Vec<String> = Vec::new();
    for n in nodes {
      if let Some(s) = self.emit_node(n) {
        parts.push(s);
      }
    }
    format!("[{}]", parts.join(", "))
  }

  fn emit_owned_children_array(&mut self, nodes: &[Node]) -> String {
    let refs: Vec<&Node> = nodes.iter().collect();
    self.emit_children_array(&refs)
  }

  /// Returns `None` for nodes that are hoisted (Import/Export/Frontmatter)
  /// or unsupported in body position (Document, Table parts).
  fn emit_node(&mut self, n: &Node) -> Option<String> {
    Some(match n {
      Node::Heading(h) => self.emit_heading(h),
      Node::Paragraph(p) => self.emit_paragraph(p),
      Node::Text(t) => Self::js_string(&t.value),
      Node::Bold(i) => self.wrap_inline("strong", &i.children, None),
      Node::Italic(i) => self.wrap_inline("em", &i.children, None),
      Node::Strikethrough(i) => self.wrap_inline("del", &i.children, None),
      Node::InlineCode(c) => {
        format!("jsx(\"code\", {{ children: {} }})", Self::js_string(&c.value))
      },
      Node::CodeBlock(cb) => self.emit_code_block(cb),
      Node::Link(l) => self.emit_link(l),
      Node::Image(i) => self.emit_image(i),
      Node::HorizontalRule(_) => "jsx(\"hr\", {})".to_string(),
      Node::Blockquote(b) => self.wrap_inline("blockquote", &b.children, None),
      Node::List(l) => self.emit_list(l),
      Node::ListItem(li) => self.wrap_inline("li", &li.children, None),
      Node::TaskListItem(t) => self.wrap_inline("li", &t.children, None),
      Node::JsxElement(e) => self.emit_jsx_element(e),
      Node::JsxSelfClosing(s) => self.emit_jsx_self_closing(s),
      Node::JsxFragment(f) => {
        let kids = self.emit_owned_children_array(&f.children);
        format!("jsxs(Fragment, {{ children: {} }})", kids)
      },
      Node::JsxExpression(j) => j.value.trim().to_string(),
      Node::HardBreak(_) => "jsx(\"br\", {})".to_string(),
      Node::SoftBreak(_) => Self::js_string("\n"),
      Node::Frontmatter(_) | Node::Import(_) | Node::Export(_) => return None,
      Node::Document(_) => return None,
      Node::Table(_) => {
        self.diag(
          Code::MdxTableUnsupported,
          "mdx-body: GFM `Table` not supported by emitter; node dropped (run `disable-gfm` to flatten tables to text)".to_string(),
        );
        return None;
      },
      Node::TableRow(_) | Node::TableCell(_) => return None,
    })
  }

  fn wrap_inline(&mut self, tag: &str, children: &[Node], extra: Option<&str>) -> String {
    let kids = self.emit_owned_children_array(children);
    match extra {
      Some(props) => format!("jsxs(\"{}\", {{ {}, children: {} }})", tag, props, kids),
      None => format!("jsxs(\"{}\", {{ children: {} }})", tag, kids),
    }
  }

  fn emit_heading(&mut self, h: &Heading) -> String {
    let kids = self.emit_owned_children_array(&h.children);
    format!("jsxs(\"h{}\", {{ id: {}, children: {} }})", h.level, Self::js_string(&h.slug()), kids)
  }

  fn emit_paragraph(&mut self, p: &Paragraph) -> String {
    let kids = self.emit_owned_children_array(&p.children);
    format!("jsxs(\"p\", {{ children: {} }})", kids)
  }

  fn emit_code_block(&mut self, cb: &CodeBlock) -> String {
    let mut props =
      format!("children: jsx(\"code\", {{ children: {} }})", Self::js_string(&cb.value));
    if let Some(lang) = &cb.lang {
      props = format!(
        "children: jsx(\"code\", {{ className: {}, children: {} }})",
        Self::js_string(&format!("gentleduck-md-language-{}", lang)),
        Self::js_string(&cb.value)
      );
    }
    format!("jsx(\"pre\", {{ {} }})", props)
  }

  fn emit_link(&mut self, l: &Link) -> String {
    let kids = self.emit_owned_children_array(&l.children);
    let mut props = format!("href: {}", Self::js_string(&l.href));
    if let Some(title) = &l.title {
      props.push_str(&format!(", \"aria-label\": {}", Self::js_string(title)));
    }
    format!("jsxs(\"a\", {{ {}, children: {} }})", props, kids)
  }

  fn emit_image(&mut self, i: &Image) -> String {
    format!(
      "jsx(\"img\", {{ src: {}, alt: {} }})",
      Self::js_string(&i.src),
      Self::js_string(&i.alt)
    )
  }

  fn emit_list(&mut self, l: &List) -> String {
    let tag = if l.ordered { "ol" } else { "ul" };
    let kids = self.emit_owned_children_array(&l.children);
    format!("jsxs(\"{}\", {{ children: {} }})", tag, kids)
  }

  fn emit_jsx_element(&mut self, e: &JsxElement) -> String {
    if e.name.is_empty() {
      self.diag(
        Code::MalformedJsxTagName,
        "mdx-body: JSX element has empty name; rendered as Fragment".to_string(),
      );
      let kids = self.emit_owned_children_array(&e.children);
      return format!("jsxs(Fragment, {{ children: {} }})", kids);
    }
    let mut props = self.emit_jsx_props(&e.attrs);
    let kids = self.emit_owned_children_array(&e.children);
    if !props.is_empty() {
      props.push_str(", ");
    }
    format!("jsxs({}, {{ {}children: {} }})", e.name, props, kids)
  }

  fn emit_jsx_self_closing(&mut self, s: &JsxSelfClosing) -> String {
    if s.name.is_empty() {
      self.diag(
        Code::MalformedJsxTagName,
        "mdx-body: self-closing JSX has empty name; emitted as null".to_string(),
      );
      return "null".to_string();
    }
    let props = self.emit_jsx_props(&s.attrs);
    format!("jsx({}, {{ {} }})", s.name, props)
  }

  fn emit_jsx_props(&mut self, attrs: &[JsxAttr]) -> String {
    let mut parts = Vec::new();
    for a in attrs {
      let key = format!("\"{}\"", a.name);
      let v = match &a.value {
        JsxAttrValue::String(s) => Self::js_string(s),
        JsxAttrValue::Expression(e) => e.trim().to_string(),
        JsxAttrValue::Boolean => "true".to_string(),
      };
      parts.push(format!("{}: {}", key, v));
    }
    parts.join(", ")
  }

  /// Quote `s` as a JS string literal. Handles `\`, `"`, `\n`, `\r`, `\t`,
  /// and any control char below 0x20 via `\uXXXX`.
  fn js_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
      match ch {
        '\\' => out.push_str("\\\\"),
        '"' => out.push_str("\\\""),
        '\n' => out.push_str("\\n"),
        '\r' => out.push_str("\\r"),
        '\t' => out.push_str("\\t"),
        c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
        c => out.push(c),
      }
    }
    out.push('"');
    out
  }
}
