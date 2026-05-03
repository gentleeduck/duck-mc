//! Render one .mdx file (or stdin) through the full pipeline and dump
//! before-render AST + after-render output (HTML and/or MDX body).
//!
//!     cargo run -p dmc-codegen --features bin --bin codegen                                   # default sample, both renderers
//!     cargo run -p dmc-codegen --features bin --bin codegen -- ../samples/headings.mdx --html
//!     echo '# hi' | cargo run -p dmc-codegen --features bin --bin codegen -- - --mdx
//!
//! Flags:
//!     --html           print HTML output (default if no renderer flag set)
//!     --mdx            print MDX body output
//!     --both           print both
//!     --tree-only      print AST only, skip rendering
//!     --no-ast         skip pre-render AST tree (default shows it)
//!     --passes <list>  comma-separated transformer names; default = with_defaults()
//!     --json           full structured dump: label, ast, html, mdx, errors (exits after)
//!     --quiet          suppress diagnostics + summary

use dmc_codegen::{HtmlEmitter, MdxBodyEmitter};
use dmc_diagnostic::metadata::{Origin, SourceMeta};
use dmc_lexer::Lexer;
use dmc_parser::Parser;
use dmc_parser::ast::*;
use dmc_transform::{
  AutolinkHeadings, BareUrlAutolink, CodeImport, ComponentPreview, ComponentSource, DisableGfm,
  Mermaid, NpmCommand, Pipeline,
};
use duck_diagnostic::DiagnosticEngine;
use serde_json::{Value, json};
use std::io::{self, Read};
use std::path::PathBuf;
use std::sync::Arc;

fn main() -> io::Result<()> {
  let mut args: Vec<String> = std::env::args().skip(1).collect();
  let mut want_html = take_flag(&mut args, "--html");
  let mut want_mdx = take_flag(&mut args, "--mdx");
  let want_both = take_flag(&mut args, "--both");
  let tree_only = take_flag(&mut args, "--tree-only");
  let no_ast = take_flag(&mut args, "--no-ast");
  let show_json = take_flag(&mut args, "--json");
  let quiet = take_flag(&mut args, "--quiet");
  let passes = take_value(&mut args, "--passes");

  if want_both {
    want_html = true;
    want_mdx = true;
  }
  if !want_html && !want_mdx && !tree_only {
    want_html = true;
  }

  let (label, source, meta) = match args.first().map(String::as_str) {
    None => {
      let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../samples/index.mdx");
      let src = std::fs::read_to_string(&path)?;
      let label = path.file_name().unwrap().to_string_lossy().into_owned();
      let meta = Arc::new(SourceMeta {
        path: Arc::from(label.clone()),
        version: 0,
        origin: Origin::File(path),
      });
      (label, src, meta)
    },
    Some("-") => {
      let mut buf = String::new();
      io::stdin().read_to_string(&mut buf)?;
      let meta =
        Arc::new(SourceMeta { path: Arc::from("<stdin>"), version: 0, origin: Origin::Stdin });
      ("<stdin>".to_string(), buf, meta)
    },
    Some(p) => {
      let path = PathBuf::from(p);
      let src = std::fs::read_to_string(&path)?;
      let label = path.file_name().unwrap().to_string_lossy().into_owned();
      let meta = Arc::new(SourceMeta {
        path: Arc::from(label.clone()),
        version: 0,
        origin: Origin::File(path),
      });
      (label, src, meta)
    },
  };

  // Single diagnostic engine threaded through every phase.
  let mut diag = DiagnosticEngine::new();

  // Lex + parse + transform.
  let mut lexer = Lexer::new(&source, meta.clone(), &mut diag);
  let _ = lexer.scan_tokens();
  let tokens = std::mem::take(&mut lexer.tokens);
  drop(lexer);
  let token_count = tokens.len();
  let mut doc = {
    let mut parser = Parser::new(tokens, meta.clone(), &mut diag);
    parser.parse()
  };
  let pipeline = match passes {
    Some(ref names) => build_pipeline_from_names(names),
    None => Pipeline::with_defaults(),
  };
  pipeline.run(&mut doc, &meta, &mut diag);

  // Each emitter owns its own diag during render; merge after.
  let html = if want_html {
    let (s, d) = HtmlEmitter::render(&doc);
    diag.extend(d);
    s
  } else {
    String::new()
  };
  let mdx = if want_mdx {
    let (s, d) = MdxBodyEmitter::render(&doc);
    diag.extend(d);
    s
  } else {
    String::new()
  };

  let total = diag.iter().count();

  // JSON: full structured dump.
  if show_json {
    let mut out = json!({
      "label": label,
      "topLevelNodes": doc.children.len(),
      "tokenCount": token_count,
      "errors": diag.error_count(),
      "warnings": diag.warning_count(),
    });
    if !no_ast {
      out["ast"] = serde_json::to_value(&doc).unwrap_or(Value::Null);
    }
    if want_html {
      out["html"] = Value::String(html.to_string());
    }
    if want_mdx {
      out["mdx"] = Value::String(mdx.to_string());
    }
    println!("{}", serde_json::to_string_pretty(&out).unwrap_or_else(|_| "{}".into()));
    return Ok(());
  }

  if !tree_only {
    println!("== {} ==", label);
  }

  if !no_ast {
    if !tree_only {
      println!("\n-- ast (post-transform) --");
    }
    print_doc(&doc);
  }

  if tree_only {
    return Ok(());
  }

  if want_html {
    println!("\n-- html --");
    println!("{}", html);
  }
  if want_mdx {
    println!("\n-- mdx body --");
    println!("{}", mdx);
  }

  if !quiet && total > 0 {
    let color = std::io::IsTerminal::is_terminal(&std::io::stdout());
    println!("\n-- diagnostics ({}) --", total);
    print!("{}", duck_diagnostic::format_all_smart(&diag, color));
  }

  if !quiet {
    println!("\nsummary: {} top-level node(s), {} diagnostic(s)", doc.children.len(), total);
  }
  Ok(())
}

fn build_pipeline_from_names(csv: &str) -> Pipeline {
  let mut p = Pipeline::new();
  for raw in csv.split(',') {
    let name = raw.trim();
    if name.is_empty() {
      continue;
    }
    p = match name {
      "autolink-headings" => p.add(AutolinkHeadings::new()),
      "bare-url" => p.add(BareUrlAutolink),
      "code-import" => p.add(CodeImport::new()),
      "component-source" => p.add(ComponentSource::default()),
      "component-preview" => p.add(ComponentPreview::default()),
      "disable-gfm" => p.add(DisableGfm),
      "mermaid" => p.add(Mermaid::default()),
      "npm-command" => p.add(NpmCommand),
      other => {
        eprintln!("warning: unknown transformer {:?} (skipped)", other);
        p
      },
    };
  }
  p
}

fn take_flag(args: &mut Vec<String>, name: &str) -> bool {
  let had = args.iter().any(|a| a == name);
  args.retain(|a| a != name);
  had
}

fn take_value(args: &mut Vec<String>, name: &str) -> Option<String> {
  let mut i = 0;
  while i < args.len() {
    if args[i] == name && i + 1 < args.len() {
      let v = args.remove(i + 1);
      args.remove(i);
      return Some(v);
    }
    if let Some(rest) = args[i].strip_prefix(&format!("{name}=")) {
      let v = rest.to_string();
      args.remove(i);
      return Some(v);
    }
    i += 1;
  }
  None
}

fn print_doc(doc: &Document) {
  println!("Document");
  for (i, child) in doc.children.iter().enumerate() {
    let last = i + 1 == doc.children.len();
    print_node(child, "", last);
  }
}

fn print_node(node: &Node, prefix: &str, last: bool) {
  let connector = if last { "└─ " } else { "├─ " };
  let child_prefix = format!("{prefix}{}", if last { "   " } else { "│  " });
  if let Node::Table(t) = node {
    let aligns: Vec<String> = t.align.iter().map(|a| format!("{:?}", a)).collect();
    println!("{prefix}{connector}Table        align=[{}]", aligns.join(", "));
    for (ri, row) in t.children.iter().enumerate() {
      let row_last = ri + 1 == t.children.len();
      let row_conn = if row_last { "└─ " } else { "├─ " };
      let cell_prefix = format!("{child_prefix}{}", if row_last { "   " } else { "│  " });
      println!("{child_prefix}{row_conn}TableRow[{ri}]");
      for (ci, cell) in row.cells.iter().enumerate() {
        let cell_last = ci + 1 == row.cells.len();
        let cell_conn = if cell_last { "└─ " } else { "├─ " };
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
    Node::Heading(h) => {
      (format!("Heading      level={} slug=\"{}\"", h.level, h.slug()), h.children.iter().collect())
    },
    Node::Paragraph(p) => ("Paragraph".to_string(), p.children.iter().collect()),
    Node::Text(t) => (format!("Text         {:?}", trunc(&t.value, 80)), vec![]),
    Node::Bold(i) => ("Bold".to_string(), i.children.iter().collect()),
    Node::Italic(i) => ("Italic".to_string(), i.children.iter().collect()),
    Node::Strikethrough(i) => ("Strikethrough".to_string(), i.children.iter().collect()),
    Node::InlineCode(c) => (format!("InlineCode   {:?}", trunc(&c.value, 80)), vec![]),
    Node::CodeBlock(b) => {
      (format!("CodeBlock    lang={:?} meta={:?} bytes={}", b.lang, b.meta, b.value.len()), vec![])
    },
    Node::Link(l) => {
      (format!("Link         href=\"{}\" title={:?}", l.href, l.title), l.children.iter().collect())
    },
    Node::Image(i) => {
      (format!("Image        src=\"{}\" alt={:?} title={:?}", i.src, i.alt, i.title), vec![])
    },
    Node::HorizontalRule(_) => ("HorizontalRule".to_string(), vec![]),
    Node::Blockquote(b) => ("Blockquote".to_string(), b.children.iter().collect()),
    Node::List(l) => (
      format!("List         ordered={} start={:?}", l.ordered, l.start),
      l.children.iter().collect(),
    ),
    Node::ListItem(i) => ("ListItem".to_string(), i.children.iter().collect()),
    Node::TaskListItem(i) => {
      (format!("TaskListItem checked={}", i.checked), i.children.iter().collect())
    },
    Node::Table(_) => unreachable!("Table is rendered inline by print_node"),
    Node::TableRow(_) | Node::TableCell(_) => ("(table piece)".to_string(), vec![]),
    Node::JsxElement(e) => {
      let attrs = e
        .attrs
        .iter()
        .map(|a| format!("{}={}", a.name, fmt_attr_value(&a.value)))
        .collect::<Vec<_>>()
        .join(" ");
      (
        format!("JsxElement   <{}{}{}>", e.name, if attrs.is_empty() { "" } else { " " }, attrs),
        e.children.iter().collect(),
      )
    },
    Node::JsxSelfClosing(e) => {
      let attrs = e
        .attrs
        .iter()
        .map(|a| format!("{}={}", a.name, fmt_attr_value(&a.value)))
        .collect::<Vec<_>>()
        .join(" ");
      (
        format!("JsxSelfClose <{}{}{} />", e.name, if attrs.is_empty() { "" } else { " " }, attrs),
        vec![],
      )
    },
    Node::JsxFragment(f) => ("JsxFragment  <>".to_string(), f.children.iter().collect()),
    Node::JsxExpression(x) => (format!("JsxExpr      {:?}", trunc(&x.value, 80)), vec![]),
    Node::HardBreak(_) => ("HardBreak".to_string(), vec![]),
    Node::SoftBreak(_) => ("SoftBreak".to_string(), vec![]),
  }
}

fn fmt_attr_value(v: &JsxAttrValue) -> String {
  match v {
    JsxAttrValue::String(s) => format!("\"{}\"", s),
    JsxAttrValue::Expression(e) => format!("{{{}}}", trunc(e, 40)),
    JsxAttrValue::Boolean => "true".to_string(),
  }
}

fn trunc(s: &str, max: usize) -> String {
  if s.chars().count() <= max {
    s.to_string()
  } else {
    let mut out: String = s.chars().take(max).collect();
    out.push('…');
    out
  }
}
