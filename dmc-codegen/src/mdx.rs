use std::collections::BTreeSet;

use crate::{NodeSink, WalkCtx, Walker, escape::sanitize_url};
use dmc_diagnostic::Code;
use dmc_parser::ast::*;
use duck_diagnostic::{DiagnosticEngine, diag};

/// Builds an MDX-runtime body: `_createMdxContent(props)` returning a tree
/// of `jsx`/`jsxs`/`Fragment`. Output shape follows `@mdx-js/mdx`'s
/// function-body format:
/// - `Fragment`/`jsx`/`jsxs` from `arguments[0]`
/// - `_components = { tag: "tag", ..., ...props.components }` -- only
///   referenced intrinsics get a default entry
/// - capitalized JSX names destructured off `_components` and validated
///   via `_missingMdxReference`
/// - `jsx` for zero/one child, `jsxs` for multiple
#[derive(Debug)]
pub struct MdxBodyEmitter {
  stack: Vec<Frame>,
  imports: Vec<String>,
  exports: Vec<String>,
  diag_engine: DiagnosticEngine<Code>,
  in_table_depth: usize,
  used_intrinsic: BTreeSet<String>,
  used_components: BTreeSet<String>,
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
        let tag = self.jsx_tag_ref("code");
        // No `__dmcRaw__` here -- that flag is reserved for fenced `<pre>`
        // blocks (PrettyCode); putting it on inline `<code>` misclassifies
        // it as block in consumer mappings.
        self.push_part(format!("jsx({}, {{ children: {} }})", tag, Self::js_string(&c.value),));
      },
      Node::CodeBlock(cb) => {
        let s = self.code_block_expr(cb);
        self.push_part(s);
      },
      Node::Image(i) => {
        let s = self.image_expr(i);
        self.push_part(s);
      },
      Node::HorizontalRule(_) => {
        let tag = self.jsx_tag_ref("hr");
        self.push_part(format!("jsx({}, {{}})", tag));
      },
      Node::HardBreak(_) => {
        let tag = self.jsx_tag_ref("br");
        self.push_part(format!("jsx({}, {{}})", tag));
      },
      Node::SoftBreak(_) => self.push_part(Self::js_string("\n")),
      Node::JsxSelfClosing(s) => {
        let expr = self.jsx_self_closing_expr(s);
        self.push_part(expr);
      },
      Node::JsxExpression(j) => self.push_part(j.value.trim().to_string()),

      // Must be an explicit arm: `_ => open_frame` would push a frame
      // but `is_container` returns false for `Html`, so `close_frame`
      // would bail without popping -- silently dropping every sibling
      // and ancestor expression that follows.
      Node::Html(h) => {
        let tag = self.jsx_tag_ref("div");
        self.push_part(format!(
          "jsx({}, {{ dangerouslySetInnerHTML: {{ __html: {} }} }})",
          tag,
          Self::js_string(&h.value)
        ));
      },

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
      used_intrinsic: BTreeSet::new(),
      used_components: BTreeSet::new(),
    }
  }

  pub fn render(doc: &Document) -> (String, DiagnosticEngine<Code>) {
    let mut emitter = Self::new();
    Walker::new(doc).walk(&mut [&mut emitter]);
    emitter.into_parts()
  }

  pub fn into_parts(mut self) -> (String, DiagnosticEngine<Code>) {
    let diag = std::mem::replace(&mut self.diag_engine, DiagnosticEngine::new());
    let body_str = self.into_string();
    (body_str, diag)
  }

  pub fn into_string(self) -> String {
    let MdxBodyEmitter { stack, imports, exports, used_intrinsic, used_components, .. } = self;
    let root_parts = stack.into_iter().next().map(|f| f.parts).unwrap_or_default();
    let (root_callee, root_kids) = jsx_callee_and_children(&root_parts);
    let body_expr = format!("{}(Fragment, {{ children: {} }})", root_callee, root_kids);

    // Function-body output (consumed via `new Function(body)(runtime)`)
    // can't legally contain `import`/`export`. Drop them; consumers that
    // need ESM bindings declare them outside MDX (e.g. components map).
    let _ = (&imports, &exports);
    let prelude = String::new();

    let defaults = if used_intrinsic.is_empty() {
      "...props.components".to_string()
    } else {
      let entries: Vec<String> = used_intrinsic.iter().map(|tag| format!("{}: \"{}\"", obj_key(tag), tag)).collect();
      format!("{}, ...props.components", entries.join(", "))
    };

    let (component_destructure, missing_checks, missing_fn) = if used_components.is_empty() {
      (String::new(), String::new(), String::new())
    } else {
      let names: Vec<String> = used_components.iter().cloned().collect();
      let destruct = format!("  const {{ {} }} = _components;\n", names.join(", "));
      let mut checks = String::new();
      for name in &names {
        checks.push_str(&format!("  if (!{name}) _missingMdxReference(\"{name}\");\n"));
      }
      let f = "function _missingMdxReference(name) { throw new Error(\"Component <\" + name + \"> was not provided via the MDX components prop. Register it in your component map.\"); }\n".to_string();
      (destruct, checks, f)
    };

    // Destructure runtime at module scope so `_createMdxContent` closes
    // over it; inside the fn it would be shadowed by React's `props`.
    format!(
      "{prelude}const {{ Fragment, jsx, jsxs }} = arguments[0];\n{missing_fn}function _createMdxContent(props) {{\n  const _components = {{ {defaults} }};\n{component_destructure}{missing_checks}  return {body_expr};\n}}\nreturn {{ default: _createMdxContent }};\n",
    )
  }

  fn diag(&mut self, code: Code, message: impl Into<String>) {
    self.diag_engine.emit(diag!(code, message.into()));
  }

  fn open_frame(&mut self, _node: &Node) {
    self.stack.push(Frame::default());
  }

  fn close_frame(&mut self, node: &Node) {
    if !Self::is_container(node) {
      return;
    }
    let kid_parts = self.pop_kid_parts();
    let (callee, kids) = jsx_callee_and_children(&kid_parts);
    let expr = match node {
      Node::Heading(h) => {
        let tag = format!("h{}", h.level);
        format!("{}({}, {{ id: {}, children: {} }})", callee, self.jsx_tag_ref(&tag), Self::js_string(&h.slug()), kids,)
      },
      Node::Paragraph(_) => format!("{}({}, {{ children: {} }})", callee, self.jsx_tag_ref("p"), kids),
      Node::Bold(_) => format!("{}({}, {{ children: {} }})", callee, self.jsx_tag_ref("strong"), kids),
      Node::Italic(_) => format!("{}({}, {{ children: {} }})", callee, self.jsx_tag_ref("em"), kids),
      Node::Strikethrough(_) => format!("{}({}, {{ children: {} }})", callee, self.jsx_tag_ref("del"), kids),
      Node::Blockquote(_) => format!("{}({}, {{ children: {} }})", callee, self.jsx_tag_ref("blockquote"), kids),
      Node::List(l) => {
        let tag = if l.ordered { "ol" } else { "ul" };
        format!("{}({}, {{ children: {} }})", callee, self.jsx_tag_ref(tag), kids)
      },
      Node::ListItem(_) | Node::TaskListItem(_) => {
        format!("{}({}, {{ children: {} }})", callee, self.jsx_tag_ref("li"), kids)
      },
      Node::Link(l) => {
        let mut props = format!("href: {}", Self::js_string(sanitize_url(&l.href)));
        if let Some(title) = &l.title {
          props.push_str(&format!(", \"aria-label\": {}", Self::js_string(title)));
        }
        format!("{}({}, {{ {}, children: {} }})", callee, self.jsx_tag_ref("a"), props, kids)
      },
      Node::JsxElement(e) => self.jsx_element_expr_with(e, callee, kids),
      Node::JsxFragment(_) => format!("{}(Fragment, {{ children: {} }})", callee, kids),
      _ => unreachable!("is_container guards every other variant"),
    };
    self.push_part(expr);
  }

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

  fn pop_kid_parts(&mut self) -> Vec<String> {
    self.stack.pop().map(|f| f.parts).unwrap_or_default()
  }

  fn push_part(&mut self, expr: String) {
    if let Some(frame) = self.stack.last_mut() {
      frame.parts.push(expr);
    }
  }

  fn code_block_expr(&mut self, cb: &CodeBlock) -> String {
    let pre = self.jsx_tag_ref("pre");
    let code = self.jsx_tag_ref("code");
    match &cb.lang {
      Some(lang) => format!(
        "jsx({}, {{ children: jsx({}, {{ className: {}, children: {} }}) }})",
        pre,
        code,
        Self::js_string(&format!("gentledmc-language-{}", lang)),
        Self::js_string(&cb.value),
      ),
      None => format!("jsx({}, {{ children: jsx({}, {{ children: {} }}) }})", pre, code, Self::js_string(&cb.value),),
    }
  }

  fn image_expr(&mut self, i: &Image) -> String {
    format!(
      "jsx({}, {{ src: {}, alt: {} }})",
      self.jsx_tag_ref("img"),
      Self::js_string(sanitize_url(&i.src)),
      Self::js_string(&i.alt)
    )
  }

  fn jsx_element_expr_with(&mut self, e: &JsxElement, callee: &str, kids: String) -> String {
    if e.name.is_empty() {
      self.diag(Code::MalformedJsxTagName, "mdx-body: JSX element has empty name; rendered as Fragment".to_string());
      return format!("{}(Fragment, {{ children: {} }})", callee, kids);
    }
    let mut props = self.jsx_props(&e.attrs);
    if !props.is_empty() {
      props.push_str(", ");
    }
    format!("{}({}, {{ {}children: {} }})", callee, self.jsx_tag_ref(&e.name), props, kids)
  }

  fn jsx_self_closing_expr(&mut self, s: &JsxSelfClosing) -> String {
    if s.name.is_empty() {
      self.diag(Code::MalformedJsxTagName, "mdx-body: self-closing JSX has empty name; emitted as null".to_string());
      return "null".to_string();
    }
    let props = self.jsx_props(&s.attrs);
    format!("jsx({}, {{ {} }})", self.jsx_tag_ref(&s.name), props)
  }

  /// Convert a CSS-style attribute string to a JSX object literal.
  /// `--custom` properties stay quoted; everything else is camel-cased.
  fn style_attr_to_object(s: &str) -> String {
    let mut entries = Vec::new();
    for decl in s.split(';') {
      let decl = decl.trim();
      if decl.is_empty() {
        continue;
      }
      let Some((raw_key, raw_val)) = decl.split_once(':') else {
        continue;
      };
      let key = raw_key.trim();
      let val = raw_val.trim();
      if key.is_empty() {
        continue;
      }
      let key_out = if key.starts_with("--") {
        format!("\"{}\"", key)
      } else {
        let mut camel = String::with_capacity(key.len());
        let mut upper = false;
        for ch in key.chars() {
          if ch == '-' {
            upper = true;
          } else if upper {
            camel.push(ch.to_ascii_uppercase());
            upper = false;
          } else {
            camel.push(ch.to_ascii_lowercase());
          }
        }
        camel
      };
      entries.push(format!("{}: {}", key_out, Self::js_string(val)));
    }
    if entries.is_empty() { "{}".to_string() } else { format!("{{ {} }}", entries.join(", ")) }
  }

  /// Resolve a JSX tag name and record the ref for the prelude.
  /// - lowercase -> `_components.<tag>`
  /// - capitalized -> bare local binding (destructured in prelude)
  /// - `Fragment` -> in-scope runtime symbol
  /// - non-ident (`my-element`) -> `_components[...]`
  fn jsx_tag_ref(&mut self, name: &str) -> String {
    if name == "Fragment" {
      return "Fragment".to_string();
    }
    let starts_upper = name.chars().next().is_some_and(|c| c.is_ascii_uppercase());
    if starts_upper {
      self.used_components.insert(name.to_string());
      return name.to_string();
    }
    self.used_intrinsic.insert(name.to_string());
    if is_js_ident(name) { format!("_components.{name}") } else { format!("_components[{}]", Self::js_string(name)) }
  }

  fn jsx_props(&mut self, attrs: &[JsxAttr]) -> String {
    let mut parts = Vec::new();
    for a in attrs {
      let key = obj_key(&a.name);
      if let JsxAttrValue::Spread(e) = &a.value {
        parts.push(format!("...{}", e.trim()));
        continue;
      }
      let v = match &a.value {
        // React requires `style` as an object literal, not a string.
        JsxAttrValue::String(s) if a.name == "style" => Self::style_attr_to_object(s),
        JsxAttrValue::String(s) => Self::js_string(s),
        JsxAttrValue::Expression(e) => Self::compile_attr_expression(self, e),
        JsxAttrValue::Boolean => "true".to_string(),
        JsxAttrValue::Spread(_) => unreachable!(),
      };
      parts.push(format!("{}: {}", key, v));
    }
    parts.join(", ")
  }

  /// Compile a `{...}` JSX attribute expression. Plain JS passes through;
  /// embedded JSX (`{<Zap />}`) is re-parsed and routed through `inline_expr`.
  fn compile_attr_expression(&mut self, e: &str) -> String {
    let trimmed = e.trim();
    if !trimmed.starts_with('<') {
      return trimmed.to_string();
    }
    let nodes = dmc_parser::parse_inline_str(trimmed);
    let pieces: Vec<String> = nodes
      .iter()
      .filter(|n| !matches!(n, Node::Text(t) if t.value.trim().is_empty()))
      .map(|n| self.inline_expr(n))
      .collect();
    match pieces.len() {
      0 => trimmed.to_string(),
      1 => pieces.into_iter().next().unwrap(),
      _ => format!("jsxs(Fragment, {{ children: [{}] }})", pieces.join(", ")),
    }
  }

  /// Rows/cells aren't `Node` variants, so walk them recursively here;
  /// `in_table_depth` suppresses the outer walker.
  fn table_expr(&mut self, t: &Table) -> String {
    let mut sections: Vec<String> = Vec::new();
    let tr = self.jsx_tag_ref("tr");
    let thead = self.jsx_tag_ref("thead");
    let tbody = self.jsx_tag_ref("tbody");
    let table = self.jsx_tag_ref("table");

    if let Some(header) = t.children.first() {
      let mut head_cells: Vec<String> = Vec::with_capacity(header.cells.len());
      for (i, cell) in header.cells.iter().enumerate() {
        let align = t.align.get(i).copied().unwrap_or(TableAlign::None);
        head_cells.push(self.table_cell_expr("th", cell, align));
      }
      let head_row = format!("jsxs({}, {{ children: [{}] }})", tr, head_cells.join(", "));
      sections.push(format!("jsxs({}, {{ children: [{}] }})", thead, head_row));
    }

    if t.children.len() > 1 {
      let mut body_rows: Vec<String> = Vec::with_capacity(t.children.len() - 1);
      for row in &t.children[1..] {
        let mut row_cells: Vec<String> = Vec::with_capacity(row.cells.len());
        for (i, cell) in row.cells.iter().enumerate() {
          let align = t.align.get(i).copied().unwrap_or(TableAlign::None);
          row_cells.push(self.table_cell_expr("td", cell, align));
        }
        body_rows.push(format!("jsxs({}, {{ children: [{}] }})", tr, row_cells.join(", ")));
      }
      sections.push(format!("jsxs({}, {{ children: [{}] }})", tbody, body_rows.join(", ")));
    }

    format!("jsxs({}, {{ children: [{}] }})", table, sections.join(", "))
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
    let tag_ref = self.jsx_tag_ref(tag);
    match align_str {
      Some(a) => format!("jsxs({}, {{ align: {}, children: {} }})", tag_ref, Self::js_string(a), kids_arr),
      None => format!("jsxs({}, {{ children: {} }})", tag_ref, kids_arr),
    }
  }

  /// Self-recursive for cell content (walker suppressed via `in_table_depth`).
  fn inline_expr(&mut self, node: &Node) -> String {
    match node {
      Node::Text(t) => Self::js_string(&t.value),
      Node::InlineCode(c) => {
        format!("jsx({}, {{ children: {} }})", self.jsx_tag_ref("code"), Self::js_string(&c.value))
      },
      Node::CodeBlock(cb) => self.code_block_expr(cb),
      Node::Image(i) => self.image_expr(i),
      Node::HorizontalRule(_) => format!("jsx({}, {{}})", self.jsx_tag_ref("hr")),
      Node::HardBreak(_) => format!("jsx({}, {{}})", self.jsx_tag_ref("br")),
      Node::SoftBreak(_) => Self::js_string("\n"),
      Node::JsxSelfClosing(s) => self.jsx_self_closing_expr(s),
      Node::JsxExpression(j) => j.value.trim().to_string(),
      Node::Bold(i) => self.wrap_jsx("strong", &i.children),
      Node::Italic(i) => self.wrap_jsx("em", &i.children),
      Node::Strikethrough(i) => self.wrap_jsx("del", &i.children),
      Node::Paragraph(p) => self.wrap_jsx("p", &p.children),
      Node::Blockquote(b) => self.wrap_jsx("blockquote", &b.children),
      Node::List(l) => {
        let tag = if l.ordered { "ol" } else { "ul" };
        self.wrap_jsx(tag, &l.children)
      },
      Node::ListItem(li) => self.wrap_jsx("li", &li.children),
      Node::TaskListItem(t) => self.wrap_jsx("li", &t.children),
      Node::Heading(h) => {
        let kids: Vec<String> = h.children.iter().map(|n| self.inline_expr(n)).collect();
        let (callee, kids_expr) = jsx_callee_and_children(&kids);
        let tag = format!("h{}", h.level);
        format!(
          "{}({}, {{ id: {}, children: {} }})",
          callee,
          self.jsx_tag_ref(&tag),
          Self::js_string(&h.slug()),
          kids_expr,
        )
      },
      Node::Link(l) => {
        let kids: Vec<String> = l.children.iter().map(|n| self.inline_expr(n)).collect();
        let (callee, kids_expr) = jsx_callee_and_children(&kids);
        let mut props = format!("href: {}", Self::js_string(sanitize_url(&l.href)));
        if let Some(title) = &l.title {
          props.push_str(&format!(", \"aria-label\": {}", Self::js_string(title)));
        }
        format!("{}({}, {{ {}, children: {} }})", callee, self.jsx_tag_ref("a"), props, kids_expr)
      },
      Node::JsxElement(e) => {
        let kids: Vec<String> = e.children.iter().map(|n| self.inline_expr(n)).collect();
        let (callee, kids_expr) = jsx_callee_and_children(&kids);
        self.jsx_element_expr_with(e, callee, kids_expr)
      },
      Node::JsxFragment(f) => {
        let kids: Vec<String> = f.children.iter().map(|n| self.inline_expr(n)).collect();
        let (callee, kids_expr) = jsx_callee_and_children(&kids);
        format!("{}(Fragment, {{ children: {} }})", callee, kids_expr)
      },
      Node::Table(t) => self.table_expr(t),
      Node::Html(h) => format!(
        "jsx({}, {{ dangerouslySetInnerHTML: {{ __html: {} }} }})",
        self.jsx_tag_ref("div"),
        Self::js_string(&h.value)
      ),
      Node::FootnoteRef(f) => format!(
        "jsx({}, {{ children: jsx({}, {{ href: \"#fn-{}\", children: {} }}) }})",
        self.jsx_tag_ref("sup"),
        self.jsx_tag_ref("a"),
        f.id,
        Self::js_string(&f.id)
      ),
      Node::FootnoteDef(f) => self.wrap_jsx("p", &f.children),
      Node::Frontmatter(_)
      | Node::Import(_)
      | Node::Export(_)
      | Node::Document(_)
      | Node::TableRow(_)
      | Node::TableCell(_) => "null".to_string(),
    }
  }

  fn wrap_jsx(&mut self, tag: &str, children: &[Node]) -> String {
    let kids: Vec<String> = children.iter().map(|n| self.inline_expr(n)).collect();
    let (callee, kids_expr) = jsx_callee_and_children(&kids);
    format!("{}({}, {{ children: {} }})", callee, self.jsx_tag_ref(tag), kids_expr)
  }

  /// Quote `s` as a JS string literal; control chars below 0x20 via `\uXXXX`.
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

pub fn render_mdx_body(doc: &Document) -> String {
  MdxBodyEmitter::render(doc).0
}

/// `@mdx-js/mdx` callee rule: 0/1 child -> `jsx` unwrapped; many -> `jsxs([])`.
/// Adjacent string literals are coalesced first -- React SSR emits a
/// `<!-- -->` between sibling text nodes, so without merging, each
/// per-word `Text` node leaks an HTML comment into the DOM.
fn jsx_callee_and_children(parts: &[String]) -> (&'static str, String) {
  let merged = coalesce_string_literals(parts);
  match merged.len() {
    0 => ("jsx", "[]".into()),
    1 => ("jsx", merged.into_iter().next().unwrap()),
    _ => ("jsxs", format!("[{}]", merged.join(", "))),
  }
}

/// Fold runs of `"..."` literals into one. Non-string expressions act as breaks.
fn coalesce_string_literals(parts: &[String]) -> Vec<String> {
  let mut out: Vec<String> = Vec::with_capacity(parts.len());
  for p in parts {
    if is_js_string_literal(p)
      && let Some(last) = out.last_mut()
      && is_js_string_literal(last)
    {
      last.pop();
      last.push_str(&p[1..]);
      continue;
    }
    out.push(p.clone());
  }
  out
}

/// True when `s` is exactly one JS string literal from `js_string`
/// (interior `"` and `\` properly escaped).
fn is_js_string_literal(s: &str) -> bool {
  let bytes = s.as_bytes();
  if bytes.len() < 2 || bytes[0] != b'"' || bytes[bytes.len() - 1] != b'"' {
    return false;
  }
  let mut i = 1;
  let end = bytes.len() - 1;
  while i < end {
    match bytes[i] {
      b'\\' => {
        if i + 1 >= end {
          return false;
        }
        i += 2;
      },
      b'"' => return false,
      _ => i += 1,
    }
  }
  true
}

/// Safe to emit unquoted as member access or object-literal key.
fn is_js_ident(s: &str) -> bool {
  let mut chars = s.chars();
  match chars.next() {
    Some(c) if c.is_ascii_alphabetic() || c == '_' || c == '$' => {},
    _ => return false,
  }
  chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
}

fn obj_key(key: &str) -> String {
  if is_js_ident(key) { key.to_string() } else { format!("\"{}\"", key.replace('"', "\\\"")) }
}
