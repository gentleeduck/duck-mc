use duck_diagnostic::Span;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Node {
  Document(Document),
  Frontmatter(Frontmatter),
  Import(Import),
  Export(Export),
  Heading(Heading),
  Paragraph(Paragraph),
  Text(Text),
  Bold(Inline),
  Italic(Inline),
  Strikethrough(Inline),
  InlineCode(InlineCode),
  CodeBlock(CodeBlock),
  Link(Link),
  Image(Image),
  HorizontalRule(HorizontalRule),
  Blockquote(Blockquote),
  List(List),
  ListItem(ListItem),
  TaskListItem(TaskListItem),
  Table(Table),
  TableRow(TableRow),
  TableCell(TableCell),
  JsxElement(JsxElement),
  JsxSelfClosing(JsxSelfClosing),
  JsxFragment(JsxFragment),
  JsxExpression(JsxExpression),
  HardBreak(BreakNode),
  SoftBreak(BreakNode),
  Html(Html),
  FootnoteRef(FootnoteRef),
  FootnoteDef(FootnoteDef),
}

impl Node {
  pub fn children_of(node: &Node) -> &[Node] {
    match node {
      Node::Document(n) => &n.children,
      Node::Heading(n) => &n.children,
      Node::Paragraph(n) => &n.children,
      Node::Bold(n) | Node::Italic(n) | Node::Strikethrough(n) => &n.children,
      Node::Link(n) => &n.children,
      Node::Blockquote(n) => &n.children,
      Node::List(n) => &n.children,
      Node::ListItem(n) => &n.children,
      Node::TaskListItem(n) => &n.children,
      Node::TableCell(n) => &n.children,
      Node::JsxElement(n) => &n.children,
      Node::JsxFragment(n) => &n.children,
      Node::FootnoteDef(n) => &n.children,
      _ => &[],
    }
  }

  pub fn children_of_mut(node: &mut Node) -> Option<&mut Vec<Node>> {
    match node {
      Node::Document(n) => Some(&mut n.children),
      Node::Heading(n) => Some(&mut n.children),
      Node::Paragraph(n) => Some(&mut n.children),
      Node::Bold(n) | Node::Italic(n) | Node::Strikethrough(n) => Some(&mut n.children),
      Node::Link(n) => Some(&mut n.children),
      Node::Blockquote(n) => Some(&mut n.children),
      Node::List(n) => Some(&mut n.children),
      Node::ListItem(n) => Some(&mut n.children),
      Node::TaskListItem(n) => Some(&mut n.children),
      Node::TableCell(n) => Some(&mut n.children),
      Node::JsxElement(n) => Some(&mut n.children),
      Node::JsxFragment(n) => Some(&mut n.children),
      Node::FootnoteDef(n) => Some(&mut n.children),
      _ => None,
    }
  }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Document {
  pub children: Vec<Node>,
  pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Frontmatter {
  pub raw: String,
  pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Import {
  pub raw: String,
  pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Export {
  pub raw: String,
  pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Heading {
  pub level: u8,
  pub children: Vec<Node>,
  pub span: Span,
  /// Pre-computed anchor id, populated by the `AssignHeadingIds` transform
  /// (github-slugger algorithm with document-wide dedupe). When `None`,
  /// `slug()` falls back to a fresh per-heading computation, so headings
  /// emitted before the transform pass still have a usable anchor.
  #[serde(default)]
  pub id: Option<String>,
}

impl Heading {
  /// URL-anchor slug. Returns the pre-computed `id` when available
  /// (preferred — only the document-scoped pass can dedupe duplicates),
  /// else a one-shot github-slugger computation from the heading text.
  pub fn slug(&self) -> String {
    if let Some(id) = &self.id {
      return id.clone();
    }
    crate::slugger::github_slugify(&Self::plain_text(&self.children))
  }

  /// Flatten inline nodes to bare text. Recurses through emphasis and link
  /// wrappers but skips JSX and images.
  pub fn plain_text(nodes: &[Node]) -> String {
    let mut s = String::new();
    for n in nodes {
      match n {
        Node::Text(t) => s.push_str(&t.value),
        Node::Bold(i) | Node::Italic(i) | Node::Strikethrough(i) => s.push_str(&Self::plain_text(&i.children)),
        Node::Link(l) => s.push_str(&Self::plain_text(&l.children)),
        Node::InlineCode(c) => s.push_str(&c.value),
        _ => {},
      }
    }
    s.trim().to_string()
  }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Paragraph {
  pub children: Vec<Node>,
  pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Text {
  pub value: String,
  pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Inline {
  pub children: Vec<Node>,
  pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InlineCode {
  pub value: String,
  pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CodeBlock {
  pub lang: Option<String>,
  pub meta: Option<String>,
  pub value: String,
  pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Link {
  pub href: String,
  pub title: Option<String>,
  pub children: Vec<Node>,
  pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Image {
  pub src: String,
  pub alt: String,
  pub title: Option<String>,
  pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HorizontalRule {
  pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Blockquote {
  pub children: Vec<Node>,
  pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct List {
  pub ordered: bool,
  pub start: Option<u32>,
  /// Children are `ListItem` or `TaskListItem` `Node` variants.
  pub children: Vec<Node>,
  pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListItem {
  pub children: Vec<Node>,
  pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskListItem {
  pub checked: bool,
  pub children: Vec<Node>,
  pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TableAlign {
  None,
  Left,
  Right,
  Center,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Table {
  pub align: Vec<TableAlign>,
  pub children: Vec<TableRow>,
  pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableRow {
  pub cells: Vec<TableCell>,
  pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableCell {
  pub children: Vec<Node>,
  pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsxElement {
  pub name: String,
  pub attrs: Vec<crate::ast::JsxAttr>,
  pub children: Vec<Node>,
  pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsxSelfClosing {
  pub name: String,
  pub attrs: Vec<crate::ast::JsxAttr>,
  pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsxFragment {
  pub children: Vec<Node>,
  pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsxExpression {
  pub value: String,
  pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BreakNode {
  pub span: Span,
}

/// Raw HTML block (CommonMark 4.6). Lexer classifies the type via
/// `HtmlBlockKind`; the parser captures the body verbatim and the
/// renderer emits it untouched per CM rules.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Html {
  pub value: String,
  pub span: Span,
}

/// GFM footnote reference (`[^id]`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FootnoteRef {
  pub id: String,
  pub span: Span,
}

/// GFM footnote definition (`[^id]: body`). The body is an inline
/// subtree; renderers number the definition globally on output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FootnoteDef {
  pub id: String,
  pub children: Vec<Node>,
  pub span: Span,
}
