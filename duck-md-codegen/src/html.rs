use duck_md_ast::*;
use crate::escape::{escape_attr, escape_text};

#[derive(Default)]
pub struct HtmlEmitter {
    out: String,
}

pub fn render_html(doc: &Document) -> String {
    let mut e = HtmlEmitter::default();
    for n in &doc.children {
        e.emit(n);
    }
    e.into_string()
}

impl HtmlEmitter {
    pub fn into_string(self) -> String { self.out }

    pub fn emit(&mut self, node: &Node) {
        match node {
            Node::Document(d) => for c in &d.children { self.emit(c); }
            Node::Frontmatter(_) => { /* not rendered into HTML */ }
            Node::Import(_) | Node::Export(_) => { /* MDX-only, not HTML */ }
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
            }
            Node::CodeBlock(cb) => self.emit_code_block(cb),
            Node::Link(l) => self.emit_link(l),
            Node::Image(i) => self.emit_image(i),
            Node::HorizontalRule(_) => self.out.push_str("<hr />"),
            Node::Blockquote(b) => self.emit_blockquote(b),
            Node::List(l) => self.emit_list(l),
            Node::ListItem(li) => self.emit_list_item(li),
            Node::TaskListItem(t) => self.emit_task_list_item(t),
            Node::Table(_) | Node::TableRow(_) | Node::TableCell(_) => {
                // tables not yet wired here; emit as-is best-effort
            }
            Node::JsxElement(e) => self.emit_jsx_element(e),
            Node::JsxSelfClosing(s) => self.emit_jsx_self_closing(s),
            Node::JsxFragment(f) => for c in &f.children { self.emit(c); }
            Node::JsxExpression(_) => { /* HTML output omits raw JS expressions */ }
            Node::HardBreak(_) => self.out.push_str("<br/>"),
            Node::SoftBreak(_) => self.out.push('\n'),
        }
    }

    fn wrap_inline(&mut self, tag: &str, children: &[Node]) {
        self.out.push('<'); self.out.push_str(tag); self.out.push('>');
        for c in children { self.emit(c); }
        self.out.push_str("</"); self.out.push_str(tag); self.out.push('>');
    }

    fn emit_heading(&mut self, h: &Heading) {
        self.out.push_str(&format!("<h{} id=\"{}\">", h.level, escape_attr(&h.id)));
        for c in &h.children { self.emit(c); }
        self.out.push_str(&format!("</h{}>", h.level));
    }

    fn emit_paragraph(&mut self, p: &Paragraph) {
        self.out.push_str("<p>");
        for c in &p.children { self.emit(c); }
        self.out.push_str("</p>");
    }

    fn emit_code_block(&mut self, cb: &CodeBlock) {
        if let Some(h) = &cb.highlighted_html {
            self.out.push_str(h);
            return;
        }
        self.out.push_str("<pre><code");
        if let Some(lang) = &cb.lang {
            self.out.push_str(&format!(" class=\"language-{}\"", escape_attr(lang)));
        }
        self.out.push('>');
        self.out.push_str(&escape_text(&cb.value));
        self.out.push_str("</code></pre>");
    }

    fn emit_link(&mut self, l: &Link) {
        self.out.push_str(&format!("<a href=\"{}\"", escape_attr(&l.href)));
        if let Some(title) = &l.title {
            self.out.push_str(&format!(" title=\"{}\"", escape_attr(title)));
        }
        self.out.push('>');
        for c in &l.children { self.emit(c); }
        self.out.push_str("</a>");
    }

    fn emit_image(&mut self, i: &Image) {
        self.out.push_str(&format!(
            "<img src=\"{}\" alt=\"{}\"",
            escape_attr(&i.src), escape_attr(&i.alt)
        ));
        if let Some(title) = &i.title {
            self.out.push_str(&format!(" title=\"{}\"", escape_attr(title)));
        }
        self.out.push_str(" />");
    }

    fn emit_blockquote(&mut self, b: &Blockquote) {
        self.out.push_str("<blockquote>");
        for c in &b.children { self.emit(c); }
        self.out.push_str("</blockquote>");
    }

    fn emit_list(&mut self, l: &List) {
        let tag = if l.ordered { "ol" } else { "ul" };
        self.out.push('<'); self.out.push_str(tag);
        if l.ordered
            && let Some(s) = l.start
            && s != 1
        {
            self.out.push_str(&format!(" start=\"{}\"", s));
        }
        self.out.push('>');
        for c in &l.children { self.emit(c); }
        self.out.push_str("</"); self.out.push_str(tag); self.out.push('>');
    }

    fn emit_list_item(&mut self, li: &ListItem) {
        self.out.push_str("<li>");
        for c in &li.children { self.emit(c); }
        self.out.push_str("</li>");
    }

    fn emit_task_list_item(&mut self, t: &TaskListItem) {
        let checked = if t.checked { " checked" } else { "" };
        self.out.push_str(&format!(
            "<li><input type=\"checkbox\" disabled{} />",
            checked
        ));
        for c in &t.children { self.emit(c); }
        self.out.push_str("</li>");
    }

    fn emit_jsx_element(&mut self, e: &JsxElement) {
        self.out.push('<');
        self.out.push_str(&e.name);
        for a in &e.attrs { self.emit_attr(a); }
        self.out.push('>');
        for c in &e.children { self.emit(c); }
        self.out.push_str("</");
        self.out.push_str(&e.name);
        self.out.push('>');
    }

    fn emit_jsx_self_closing(&mut self, s: &JsxSelfClosing) {
        self.out.push('<');
        self.out.push_str(&s.name);
        for a in &s.attrs { self.emit_attr(a); }
        self.out.push_str(" />");
    }

    fn emit_attr(&mut self, a: &JsxAttr) {
        self.out.push(' ');
        self.out.push_str(&a.name);
        match &a.value {
            JsxAttrValue::Boolean => {}
            JsxAttrValue::String(s) => {
                self.out.push_str(&format!("=\"{}\"", escape_attr(s)));
            }
            JsxAttrValue::Expression(e) => {
                self.out.push_str(&format!("={{{}}}", e));
            }
        }
    }
}
