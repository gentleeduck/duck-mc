use dmc_diagnostic::Code;
use dmc_parser::ast::*;
use duck_diagnostic::{Diagnostic, DiagnosticEngine};

use crate::{NodeSink, WalkCtx, Walker};

/// Builds an MDX-runtime body - a `_createMdxContent(props)` function whose
/// return value is a React tree of `jsx`, `jsxs`, and `Fragment`. Imports +
/// exports hoist into a prelude; frontmatter is dropped.
///
/// Every container node is one expression with its kids inlined as a
/// comma-joined array, so we can't interleave open/close text the way
/// HTML does. Instead each container `open_frame` pushes a child Frame;
/// kid expressions accumulate there as the walker descends; `close_frame`
/// pops, builds the parent expression, and folds it into the grandparent
/// frame.
///
/// Tables are emitted in one shot from `enter Table` (rows + cells aren't
/// walker-visible Node variants); `in_table_depth` suppresses subsequent
/// walker events on cell content.
///
/// Owns its own `DiagnosticEngine` during the walk; merge into the
/// caller's engine via `into_parts` after the walk completes.
#[derive(Debug)]
pub struct MdxBodyEmitter {
  stack: Vec<Frame>,
  imports: Vec<String>,
  exports: Vec<String>,
  diag_engine: DiagnosticEngine<Code>,
  in_table_depth: usize,
}

#[derive(Default, Debug)]
struct Frame {
  parts: Vec<String>,
}

impl NodeSink for MdxBodyEmitter {
  fn enter(&mut self, node: &Node, _ctx: &WalkCtx) {
    if self.in_table_depth > 0 {
      return;
    }
    match node {
      Node::Text(t) => self.push_part(Self::js_string(&t.value)),
      Node::InlineCode(c) => {
        self.push_part(format!("jsx(\"code\", {{ children: {} }})", Self::js_string(&c.value)));
      },
      Node::CodeBlock(cb) => self.push_part(self.code_block_expr(cb)),
      Node::Image(i) => self.push_part(self.image_expr(i)),
      Node::HorizontalRule(_) => self.push_part("jsx(\"hr\", {})".to_string()),
      Node::HardBreak(_) => self.push_part("jsx(\"br\", {})".to_string()),
      Node::SoftBreak(_) => self.push_part(Self::js_string("\n")),
      Node::JsxSelfClosing(s) => {
        let expr = self.jsx_self_closing_expr(s);
        self.push_part(expr);
      },
      Node::JsxExpression(j) => self.push_part(j.value.trim().to_string()),

      Node::Table(t) => {
        let expr = self.table_expr(t);
        self.push_part(expr);
        self.in_table_depth += 1;
      },

      Node::Frontmatter(_) => {},
      Node::Import(i) => self.imports.push(i.raw.trim_end().to_string()),
      Node::Export(x) => self.exports.push(x.raw.trim_end().to_string()),

      _ => self.open_frame(node),
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
    self.close_frame(node);
  }
}

impl Default for MdxBodyEmitter {
  fn default() -> Self {
    Self::new()
  }
}

impl MdxBodyEmitter {
  pub fn new() -> Self {
    Self {
      stack: vec![Frame::default()],
      imports: Vec::new(),
      exports: Vec::new(),
      diag_engine: DiagnosticEngine::new(),
      in_table_depth: 0,
    }
  }

  /// Drive the walker; return `(body, diag)`. Use when no other sink
  /// shares the walk.
  pub fn render(doc: &Document) -> (String, DiagnosticEngine<Code>) {
    let mut emitter = Self::new();
    Walker::new(doc).walk(&mut [&mut emitter]);
    emitter.into_parts()
  }

  /// Take both buffers: the rendered MDX body and the per-emitter
  /// diagnostic engine. Caller merges via `outer.extend(diag)`.
  pub fn into_parts(self) -> (String, DiagnosticEngine<Code>) {
    let diag = self.diag_engine;
    let body_str = Self::assemble(self.stack, self.imports, self.exports);
    (body_str, diag)
  }

  fn assemble(stack: Vec<Frame>, imports: Vec<String>, exports: Vec<String>) -> String {
    let root_parts = stack.into_iter().next().map(|f| f.parts).unwrap_or_default();
    let body = format!("jsxs(Fragment, {{ children: [{}] }})", root_parts.join(", "));
    let mut prelude = String::new();
    for i in &imports {
      prelude.push_str(i);
      prelude.push('\n');
    }
    for e in &exports {
      prelude.push_str(e);
      prelude.push('\n');
    }
    format!(
      "{prelude}function _createMdxContent(props) {{\n  const _components = (props && props.components) || {{}};\n  const {{ Fragment, jsx, jsxs }} = arguments[0];\n  return {body};\n}}\nreturn _createMdxContent(arguments[0]);\n",
    )
  }

  /// Wrap the accumulated body in the `_createMdxContent` shell and
  /// prepend the import / export prelude. Drops the diagnostic engine.
  pub fn into_string(self) -> String {
    Self::assemble(self.stack, self.imports, self.exports)
  }

  fn diag(&mut self, code: Code, message: impl Into<String>) {
    self.diag_engine.emit(Diagnostic::new(code, message.into()));
  }

  // --- frame open / close (walker fills children between) ---------------

  /// Push an empty child-frame; walker descent will populate it.
  fn open_frame(&mut self, _node: &Node) {
    self.stack.push(Frame::default());
  }

  /// Pop the current frame, build this node's expression, fold it into
  /// the parent frame. Only container variants own a frame to pop.
  fn close_frame(&mut self, node: &Node) {
    if !Self::is_container(node) {
      return;
    }
    let kids = self.pop_kids_array();
    let expr = match node {
      Node::Heading(h) => {
        format!("jsxs(\"h{}\", {{ id: {}, children: {} }})", h.level, Self::js_string(&h.slug()), kids,)
      },
      Node::Paragraph(_) => format!("jsxs(\"p\", {{ children: {} }})", kids),
      Node::Bold(_) => format!("jsxs(\"strong\", {{ children: {} }})", kids),
      Node::Italic(_) => format!("jsxs(\"em\", {{ children: {} }})", kids),
      Node::Strikethrough(_) => format!("jsxs(\"del\", {{ children: {} }})", kids),
      Node::Blockquote(_) => format!("jsxs(\"blockquote\", {{ children: {} }})", kids),
      Node::List(l) => {
        let tag = if l.ordered { "ol" } else { "ul" };
        format!("jsxs(\"{}\", {{ children: {} }})", tag, kids)
      },
      Node::ListItem(_) | Node::TaskListItem(_) => format!("jsxs(\"li\", {{ children: {} }})", kids),
      Node::Link(l) => {
        let mut props = format!("href: {}", Self::js_string(&l.href));
        if let Some(title) = &l.title {
          props.push_str(&format!(", \"aria-label\": {}", Self::js_string(title)));
        }
        format!("jsxs(\"a\", {{ {}, children: {} }})", props, kids)
      },
      Node::JsxElement(e) => self.jsx_element_expr(e, kids),
      Node::JsxFragment(_) => format!("jsxs(Fragment, {{ children: {} }})", kids),
      _ => unreachable!("is_container guards every other variant"),
    };
    self.push_part(expr);
  }

  /// True when `enter` pushed a frame for this node.
  fn is_container(n: &Node) -> bool {
    matches!(
      n,
      Node::Heading(_)
        | Node::Paragraph(_)
        | Node::Bold(_)
        | Node::Italic(_)
        | Node::Strikethrough(_)
        | Node::Blockquote(_)
        | Node::List(_)
        | Node::ListItem(_)
        | Node::TaskListItem(_)
        | Node::Link(_)
        | Node::JsxElement(_)
        | Node::JsxFragment(_)
    )
  }

  /// Pop the top frame and render its parts as a `[a, b, c]` JS array.
  fn pop_kids_array(&mut self) -> String {
    let parts = self.stack.pop().map(|f| f.parts).unwrap_or_default();
    format!("[{}]", parts.join(", "))
  }

  /// Append one expression to the current top-of-stack frame.
  fn push_part(&mut self, expr: String) {
    if let Some(frame) = self.stack.last_mut() {
      frame.parts.push(expr);
    }
  }

  // --- expression builders for leaves + cell-recursive descent ---------

  fn code_block_expr(&self, cb: &CodeBlock) -> String {
    match &cb.lang {
      Some(lang) => format!(
        "jsx(\"pre\", {{ children: jsx(\"code\", {{ className: {}, children: {} }}) }})",
        Self::js_string(&format!("gentledmc-language-{}", lang)),
        Self::js_string(&cb.value),
      ),
      None => format!("jsx(\"pre\", {{ children: jsx(\"code\", {{ children: {} }}) }})", Self::js_string(&cb.value),),
    }
  }

  fn image_expr(&self, i: &Image) -> String {
    format!("jsx(\"img\", {{ src: {}, alt: {} }})", Self::js_string(&i.src), Self::js_string(&i.alt))
  }

  fn jsx_element_expr(&mut self, e: &JsxElement, kids: String) -> String {
    if e.name.is_empty() {
      self.diag(Code::MalformedJsxTagName, "mdx-body: JSX element has empty name; rendered as Fragment".to_string());
      return format!("jsxs(Fragment, {{ children: {} }})", kids);
    }
    let mut props = self.jsx_props(&e.attrs);
    if !props.is_empty() {
      props.push_str(", ");
    }
    format!("jsxs({}, {{ {}children: {} }})", e.name, props, kids)
  }

  fn jsx_self_closing_expr(&mut self, s: &JsxSelfClosing) -> String {
    if s.name.is_empty() {
      self.diag(Code::MalformedJsxTagName, "mdx-body: self-closing JSX has empty name; emitted as null".to_string());
      return "null".to_string();
    }
    let props = self.jsx_props(&s.attrs);
    format!("jsx({}, {{ {} }})", s.name, props)
  }

  fn jsx_props(&self, attrs: &[JsxAttr]) -> String {
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

  // --- table inline path (walker can't surface row/cell events) --------

  /// Build the full `jsxs("table", { children: [thead, tbody] })` expr.
  fn table_expr(&mut self, t: &Table) -> String {
    let mut sections: Vec<String> = Vec::new();

    if let Some(header) = t.children.first() {
      let mut head_cells: Vec<String> = Vec::with_capacity(header.cells.len());
      for (i, cell) in header.cells.iter().enumerate() {
        let align = t.align.get(i).copied().unwrap_or(TableAlign::None);
        head_cells.push(self.table_cell_expr("th", cell, align));
      }
      let head_row = format!("jsxs(\"tr\", {{ children: [{}] }})", head_cells.join(", "));
      sections.push(format!("jsxs(\"thead\", {{ children: [{}] }})", head_row));
    }

    if t.children.len() > 1 {
      let mut body_rows: Vec<String> = Vec::with_capacity(t.children.len() - 1);
      for row in &t.children[1..] {
        let mut row_cells: Vec<String> = Vec::with_capacity(row.cells.len());
        for (i, cell) in row.cells.iter().enumerate() {
          let align = t.align.get(i).copied().unwrap_or(TableAlign::None);
          row_cells.push(self.table_cell_expr("td", cell, align));
        }
        body_rows.push(format!("jsxs(\"tr\", {{ children: [{}] }})", row_cells.join(", ")));
      }
      sections.push(format!("jsxs(\"tbody\", {{ children: [{}] }})", body_rows.join(", ")));
    }

    format!("jsxs(\"table\", {{ children: [{}] }})", sections.join(", "))
  }

  fn table_cell_expr(&mut self, tag: &str, cell: &TableCell, align: TableAlign) -> String {
    let kids: Vec<String> = cell.children.iter().map(|n| self.inline_expr(n)).collect();
    let kids_arr = format!("[{}]", kids.join(", "));
    let align_str = match align {
      TableAlign::Left => Some("left"),
      TableAlign::Right => Some("right"),
      TableAlign::Center => Some("center"),
      TableAlign::None => None,
    };
    match align_str {
      Some(a) => format!("jsxs(\"{}\", {{ align: {}, children: {} }})", tag, Self::js_string(a), kids_arr,),
      None => format!("jsxs(\"{}\", {{ children: {} }})", tag, kids_arr),
    }
  }

  /// Self-recursive expression builder for cell content. Walker is
  /// suppressed via `in_table_depth` while we're inside a table.
  fn inline_expr(&mut self, node: &Node) -> String {
    match node {
      Node::Text(t) => Self::js_string(&t.value),
      Node::InlineCode(c) => format!("jsx(\"code\", {{ children: {} }})", Self::js_string(&c.value)),
      Node::CodeBlock(cb) => self.code_block_expr(cb),
      Node::Image(i) => self.image_expr(i),
      Node::HorizontalRule(_) => "jsx(\"hr\", {})".to_string(),
      Node::HardBreak(_) => "jsx(\"br\", {})".to_string(),
      Node::SoftBreak(_) => Self::js_string("\n"),
      Node::JsxSelfClosing(s) => self.jsx_self_closing_expr(s),
      Node::JsxExpression(j) => j.value.trim().to_string(),
      Node::Bold(i) => self.wrap_jsxs("strong", &i.children),
      Node::Italic(i) => self.wrap_jsxs("em", &i.children),
      Node::Strikethrough(i) => self.wrap_jsxs("del", &i.children),
      Node::Paragraph(p) => self.wrap_jsxs("p", &p.children),
      Node::Blockquote(b) => self.wrap_jsxs("blockquote", &b.children),
      Node::List(l) => {
        let tag = if l.ordered { "ol" } else { "ul" };
        self.wrap_jsxs(tag, &l.children)
      },
      Node::ListItem(li) => self.wrap_jsxs("li", &li.children),
      Node::TaskListItem(t) => self.wrap_jsxs("li", &t.children),
      Node::Heading(h) => {
        let kids: Vec<String> = h.children.iter().map(|n| self.inline_expr(n)).collect();
        format!("jsxs(\"h{}\", {{ id: {}, children: [{}] }})", h.level, Self::js_string(&h.slug()), kids.join(", "),)
      },
      Node::Link(l) => {
        let kids: Vec<String> = l.children.iter().map(|n| self.inline_expr(n)).collect();
        let mut props = format!("href: {}", Self::js_string(&l.href));
        if let Some(title) = &l.title {
          props.push_str(&format!(", \"aria-label\": {}", Self::js_string(title)));
        }
        format!("jsxs(\"a\", {{ {}, children: [{}] }})", props, kids.join(", "))
      },
      Node::JsxElement(e) => {
        let kids: Vec<String> = e.children.iter().map(|n| self.inline_expr(n)).collect();
        let kids_arr = format!("[{}]", kids.join(", "));
        self.jsx_element_expr(e, kids_arr)
      },
      Node::JsxFragment(f) => {
        let kids: Vec<String> = f.children.iter().map(|n| self.inline_expr(n)).collect();
        format!("jsxs(Fragment, {{ children: [{}] }})", kids.join(", "))
      },
      Node::Table(t) => self.table_expr(t),
      Node::Frontmatter(_)
      | Node::Import(_)
      | Node::Export(_)
      | Node::Document(_)
      | Node::TableRow(_)
      | Node::TableCell(_) => "null".to_string(),
    }
  }

  fn wrap_jsxs(&mut self, tag: &str, children: &[Node]) -> String {
    let kids: Vec<String> = children.iter().map(|n| self.inline_expr(n)).collect();
    format!("jsxs(\"{}\", {{ children: [{}] }})", tag, kids.join(", "))
  }

  // --- string literal helper -------------------------------------------

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

/// Convenience: render `doc` to an MDX body string with a throwaway
/// diagnostic engine.
pub fn render_mdx_body(doc: &Document) -> String {
  MdxBodyEmitter::render(doc).0
}
