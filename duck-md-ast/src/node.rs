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
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Document {
  pub children: Vec<Node>,
  #[serde(skip, default = "crate::default_span")]
  pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Frontmatter {
  pub raw: String,
  pub data: serde_json::Value,
  #[serde(skip, default = "crate::default_span")]
  pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Import {
  pub raw: String,
  #[serde(skip, default = "crate::default_span")]
  pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Export {
  pub raw: String,
  #[serde(skip, default = "crate::default_span")]
  pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Heading {
  pub level: u8,
  pub children: Vec<Node>,
  pub id: String,
  #[serde(skip, default = "crate::default_span")]
  pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Paragraph {
  pub children: Vec<Node>,
  #[serde(skip, default = "crate::default_span")]
  pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Text {
  pub value: String,
  #[serde(skip, default = "crate::default_span")]
  pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Inline {
  pub children: Vec<Node>,
  #[serde(skip, default = "crate::default_span")]
  pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InlineCode {
  pub value: String,
  #[serde(skip, default = "crate::default_span")]
  pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CodeBlock {
  pub lang: Option<String>,
  pub meta: Option<String>,
  pub value: String,
  #[serde(skip, default = "crate::default_span")]
  pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Link {
  pub href: String,
  pub title: Option<String>,
  pub children: Vec<Node>,
  #[serde(skip, default = "crate::default_span")]
  pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Image {
  pub src: String,
  pub alt: String,
  pub title: Option<String>,
  #[serde(skip, default = "crate::default_span")]
  pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HorizontalRule {
  #[serde(skip, default = "crate::default_span")]
  pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Blockquote {
  pub children: Vec<Node>,
  #[serde(skip, default = "crate::default_span")]
  pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct List {
  pub ordered: bool,
  pub start: Option<u32>,
  /// Children are `ListItem` or `TaskListItem` `Node` variants.
  pub children: Vec<Node>,
  #[serde(skip, default = "crate::default_span")]
  pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListItem {
  pub children: Vec<Node>,
  #[serde(skip, default = "crate::default_span")]
  pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskListItem {
  pub checked: bool,
  pub children: Vec<Node>,
  #[serde(skip, default = "crate::default_span")]
  pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
  #[serde(skip, default = "crate::default_span")]
  pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableRow {
  pub cells: Vec<TableCell>,
  #[serde(skip, default = "crate::default_span")]
  pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableCell {
  pub children: Vec<Node>,
  #[serde(skip, default = "crate::default_span")]
  pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsxElement {
  pub name: String,
  pub attrs: Vec<crate::JsxAttr>,
  pub children: Vec<Node>,
  #[serde(skip, default = "crate::default_span")]
  pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsxSelfClosing {
  pub name: String,
  pub attrs: Vec<crate::JsxAttr>,
  #[serde(skip, default = "crate::default_span")]
  pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsxFragment {
  pub children: Vec<Node>,
  #[serde(skip, default = "crate::default_span")]
  pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsxExpression {
  pub value: String,
  #[serde(skip, default = "crate::default_span")]
  pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BreakNode {
  #[serde(skip, default = "crate::default_span")]
  pub span: Span,
}
