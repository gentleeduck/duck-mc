//! Parse one .mdx file (or stdin) and print the AST as an indented tree.
//!
//!     cargo run -p dmc-parser --bin parse                        # default: ../samples/index.mdx
//!     cargo run -p dmc-parser --bin parse -- path/to/file.mdx
//!     echo '# hi' | cargo run -p dmc-parser --bin parse -        # stdin (use `-`)
//!
//! Flags:
//!     --tree       print only the AST tree (no header/diagnostics/summary)
//!     --tree-only  alias of --tree
//!     --debug      print Debug `{:#?}` form alongside the tree
//!     --tokens     dump the token stream (table, or array under .tokens in --json)
//!     --json       full structured dump: label, ast, errors, warnings (exits after)
//!     --quiet      suppress diagnostics + summary text (use with --tree)

use dmc_diagnostic::metadata::{Origin, SourceMeta};
use dmc_lexer::Lexer;
use dmc_parser::Parser;
use dmc_parser::ast::*;
use duck_diagnostic::DiagnosticEngine;
use serde_json::{Value, json};
use std::io::{self, Read};
use std::path::PathBuf;
use std::sync::Arc;

fn main() -> io::Result<()> {
  let mut args: Vec<String> = std::env::args().skip(1).collect();
  let show_debug = take_flag(&mut args, "--debug");
  let show_tokens = take_flag(&mut args, "--tokens");
  let show_json = take_flag(&mut args, "--json");
  let quiet = take_flag(&mut args, "--quiet");
  let tree_only = take_flag(&mut args, "--tree-only") || take_flag(&mut args, "--tree");

  let (label, source, meta) = match args.first().map(String::as_str) {
    None => {
      let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../samples/index.mdx");
      let src = std::fs::read_to_string(&path)?;
      let label = path.file_name().unwrap().to_string_lossy().into_owned();
      let meta = Arc::new(SourceMeta { path: Arc::from(label.clone()), origin: Origin::File(path) });
      (label, src, meta)
    },
    Some("-") => {
      let mut buf = String::new();
      io::stdin().read_to_string(&mut buf)?;
      let meta = Arc::new(SourceMeta { path: Arc::from("<stdin>"), origin: Origin::Stdin });
      ("<stdin>".to_string(), buf, meta)
    },
    Some(p) => {
      let path = PathBuf::from(p);
      let src = std::fs::read_to_string(&path)?;
      let label = path.file_name().unwrap().to_string_lossy().into_owned();
      let meta = Arc::new(SourceMeta { path: Arc::from(label.clone()), origin: Origin::File(path) });
      (label, src, meta)
    },
  };

  // Single diagnostic engine threaded through every phase.
  let mut diag = DiagnosticEngine::new();

  // Lex.
  let mut lexer = Lexer::new(&source, meta.clone(), &mut diag);
  let _ = lexer.scan_tokens();
  let tokens = std::mem::take(&mut lexer.tokens);
  drop(lexer);

  // Parse.
  let doc = {
    let mut parser = Parser::new(tokens.clone(), meta.clone(), &mut diag);
    parser.parse()
  };

  let lex_errs = diag.error_count();
  let lex_warns = diag.warning_count();
  let parse_errs = 0;
  let parse_warns = 0;
  let total_diags = diag.iter().count();

  // JSON mode -> full structured dump, exit early.
  if show_json {
    let mut out = json!({
      "label": label,
      "topLevelNodes": doc.children.len(),
      "tokenCount": tokens.len(),
      "errors": lex_errs + parse_errs,
      "warnings": lex_warns + parse_warns,
      "ast": doc,
    });
    if show_tokens {
      let toks: Vec<Value> = tokens
        .iter()
        .enumerate()
        .map(|(i, t)| {
          json!({
            "index": i,
            "kind": format!("{:?}", t.kind),
            "raw": t.raw,
            "span": {
              "file": t.span.file,
              "line": t.span.line,
              "column": t.span.column,
              "length": t.span.length,
            },
          })
        })
        .collect();
      out["tokens"] = Value::Array(toks);
    }
    println!("{}", serde_json::to_string_pretty(&out).unwrap_or_else(|_| "{}".into()));
    return Ok(());
  }

  if !tree_only {
    println!("== {} ==", label);
  }

  if show_tokens && !tree_only {
    println!("\n-- tokens ({}) --", tokens.len());
    for (i, t) in tokens.iter().enumerate() {
      let raw = t.raw.replace('\n', "\\n").replace('\t', "\\t");
      println!("  [{:>4}] {:?} @ {}:{} len={}  {:?}", i, t.kind, t.span.line, t.span.column, t.span.length, raw);
    }
  }

  if !tree_only {
    println!("\n-- ast tree --");
  }
  print_doc(&doc);

  if tree_only {
    return Ok(());
  }

  if !quiet && total_diags > 0 {
    let color = std::io::IsTerminal::is_terminal(&std::io::stdout());
    println!("\n-- diagnostics ({}) --", total_diags);
    print!("{}", duck_diagnostic::format_all_smart(&diag, color));
  }

  if show_debug {
    println!("\n-- debug --\n{:#?}", doc);
  }

  if !quiet {
    println!("\nsummary: {} top-level node(s), {} diagnostic(s)", doc.children.len(), total_diags);
  }
  Ok(())
}

fn take_flag(args: &mut Vec<String>, name: &str) -> bool {
  let had = args.iter().any(|a| a == name);
  args.retain(|a| a != name);
  had
}

fn print_doc(doc: &Document) {
  println!("Document");
  for (i, child) in doc.children.iter().enumerate() {
    let last = i + 1 == doc.children.len();
    print_node(child, "", last);
  }
}

/// Box-drawing tree print with stable per-node summaries.
fn print_node(node: &Node, prefix: &str, last: bool) {
  let connector = if last { "`- " } else { "|- " };
  let child_prefix = format!("{prefix}{}", if last { "   " } else { "|  " });

  // Tables don't fit the generic Node-children walk because TableRow/TableCell
  // aren't Node variants - render them inline.
  if let Node::Table(t) = node {
    let aligns: Vec<String> = t.align.iter().map(|a| format!("{:?}", a)).collect();
    println!("{prefix}{connector}Table        align=[{}]", aligns.join(", "));
    for (ri, row) in t.children.iter().enumerate() {
      let row_last = ri + 1 == t.children.len();
      let row_conn = if row_last { "`- " } else { "|- " };
      let cell_prefix = format!("{child_prefix}{}", if row_last { "   " } else { "|  " });
      println!("{child_prefix}{row_conn}TableRow[{ri}]");
      for (ci, cell) in row.cells.iter().enumerate() {
        let cell_last = ci + 1 == row.cells.len();
        let cell_conn = if cell_last { "`- " } else { "|- " };
        let text: String = cell
          .children
          .iter()
          .filter_map(|c| match c {
            Node::Text(t) => Some(t.value.as_str()),
            _ => None,
          })
          .collect::<Vec<_>>()
          .join("");
        println!("{cell_prefix}{cell_conn}TableCell[{ci}] {:?}", trunc(&text, 60));
      }
    }
    return;
  }

  let (label, children): (String, Vec<&Node>) = describe(node);
  println!("{prefix}{connector}{label}");
  for (i, c) in children.iter().enumerate() {
    let l = i + 1 == children.len();
    print_node(c, &child_prefix, l);
  }
}

fn describe(node: &Node) -> (String, Vec<&Node>) {
  match node {
    Node::Document(d) => ("Document".to_string(), d.children.iter().collect()),
    Node::Frontmatter(f) => (format!("Frontmatter  raw={:?}", trunc(&f.raw, 80)), vec![]),
    Node::Import(i) => (format!("Import       raw={:?}", trunc(&i.raw, 80)), vec![]),
    Node::Export(e) => (format!("Export       raw={:?}", trunc(&e.raw, 80)), vec![]),
    Node::Heading(h) => (format!("Heading      level={} slug=\"{}\"", h.level, h.slug()), h.children.iter().collect()),
    Node::Paragraph(p) => ("Paragraph".to_string(), p.children.iter().collect()),
    Node::Text(t) => (format!("Text         {:?}", trunc(&t.value, 80)), vec![]),
    Node::Bold(i) => ("Bold".to_string(), i.children.iter().collect()),
    Node::Italic(i) => ("Italic".to_string(), i.children.iter().collect()),
    Node::Strikethrough(i) => ("Strikethrough".to_string(), i.children.iter().collect()),
    Node::InlineCode(c) => (format!("InlineCode   {:?}", trunc(&c.value, 80)), vec![]),
    Node::CodeBlock(b) => (format!("CodeBlock    lang={:?} meta={:?} bytes={}", b.lang, b.meta, b.value.len()), vec![]),
    Node::Link(l) => (format!("Link         href=\"{}\" title={:?}", l.href, l.title), l.children.iter().collect()),
    Node::Image(i) => (format!("Image        src=\"{}\" alt={:?} title={:?}", i.src, i.alt, i.title), vec![]),
    Node::HorizontalRule(_) => ("HorizontalRule".to_string(), vec![]),
    Node::Blockquote(b) => ("Blockquote".to_string(), b.children.iter().collect()),
    Node::List(l) => (format!("List         ordered={} start={:?}", l.ordered, l.start), l.children.iter().collect()),
    Node::ListItem(i) => ("ListItem".to_string(), i.children.iter().collect()),
    Node::TaskListItem(i) => (format!("TaskListItem checked={}", i.checked), i.children.iter().collect()),
    Node::Table(_) => unreachable!("Table is rendered inline by print_node"),
    Node::TableRow(_) | Node::TableCell(_) => ("(table piece)".to_string(), vec![]),
    Node::JsxElement(e) => {
      let attrs =
        e.attrs.iter().map(|a| format!("{}={}", a.name, fmt_attr_value(&a.value))).collect::<Vec<_>>().join(" ");
      (
        format!("JsxElement   <{}{}{}>", e.name, if attrs.is_empty() { "" } else { " " }, attrs),
        e.children.iter().collect(),
      )
    },
    Node::JsxSelfClosing(e) => {
      let attrs =
        e.attrs.iter().map(|a| format!("{}={}", a.name, fmt_attr_value(&a.value))).collect::<Vec<_>>().join(" ");
      (format!("JsxSelfClose <{}{}{} />", e.name, if attrs.is_empty() { "" } else { " " }, attrs), vec![])
    },
    Node::JsxFragment(f) => ("JsxFragment  <>".to_string(), f.children.iter().collect()),
    Node::JsxExpression(x) => (format!("JsxExpr      {:?}", trunc(&x.value, 80)), vec![]),
    Node::HardBreak(_) => ("HardBreak".to_string(), vec![]),
    Node::SoftBreak(_) => ("SoftBreak".to_string(), vec![]),
    Node::Html(h) => (format!("Html         {:?}", trunc(&h.value, 80)), vec![]),
  }
}

fn fmt_attr_value(v: &JsxAttrValue) -> String {
  match v {
    JsxAttrValue::String(s) => format!("\"{}\"", s),
    JsxAttrValue::Expression(e) => format!("{{{}}}", trunc(e, 40)),
    JsxAttrValue::Boolean => "true".to_string(),
    JsxAttrValue::Spread(e) => format!("{{...{}}}", trunc(e, 40)),
  }
}

fn trunc(s: &str, max: usize) -> String {
  if s.chars().count() <= max {
    s.to_string()
  } else {
    let mut out: String = s.chars().take(max).collect();
    out.push_str("...");
    out
  }
}
