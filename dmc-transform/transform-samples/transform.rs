//! Run the transform pipeline against one .mdx file (or stdin) and dump
//! before/after AST trees so transformer effects are visible at a glance.
//!
//!     cargo run -p dmc-transform --bin transform                          # default: ../samples/index.mdx
//!     cargo run -p dmc-transform --bin transform -- ../samples/headings.mdx
//!     echo '# hi' | cargo run -p dmc-transform --bin transform -- -       # stdin (use `-`)
//!
//! Flags:
//!     --before-only    print AST after parse, skip transform
//!     --after-only     print AST after transform, skip pre-transform
//!     --diff           print both pre + post (default)
//!     --passes <list>  comma-separated transformer names: autolink-headings,
//!                      bare-url, code-import, component-source,
//!                      component-preview, disable-gfm. Default: with_defaults().
//!     --list           list available transformers and exit
//!     --tree-only      print only AST trees (no headers, no diagnostics, no summary)
//!     --debug          print Debug `{:#?}` form alongside the tree
//!     --tokens         dump the token stream (table, or under .tokens in --json)
//!     --json           full structured dump: label, passes, tokens, before, after (exits after)
//!     --quiet          suppress diagnostics + summary text

use dmc_diagnostic::metadata::{Origin, SourceMeta};
use dmc_lexer::Lexer;
use dmc_parser::Parser;
use dmc_parser::ast::*;
use dmc_transform::{
  AutolinkHeadings, BareUrlAutolink, CodeImport, ComponentPreview, ComponentSource, DisableGfm,
  Pipeline,
};
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
  let tree_only = take_flag(&mut args, "--tree-only");
  let before_only = take_flag(&mut args, "--before-only");
  let after_only = take_flag(&mut args, "--after-only");
  let diff = take_flag(&mut args, "--diff");
  let list = take_flag(&mut args, "--list");
  let passes = take_value(&mut args, "--passes");

  if list {
    print_pass_list();
    return Ok(());
  }

  let (label, source, _input_dir, meta) = match args.first().map(String::as_str) {
    None => {
      let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../samples/index.mdx");
      let src = std::fs::read_to_string(&path)?;
      let dir = path.parent().map(|p| p.to_path_buf());
      let label = path.file_name().unwrap().to_string_lossy().into_owned();
      let meta = Arc::new(SourceMeta {
        path: Arc::from(label.clone()),
        version: 0,
        origin: Origin::File(path),
      });
      (label, src, dir, meta)
    },
    Some("-") => {
      let mut buf = String::new();
      io::stdin().read_to_string(&mut buf)?;
      let meta =
        Arc::new(SourceMeta { path: Arc::from("<stdin>"), version: 0, origin: Origin::Stdin });
      ("<stdin>".to_string(), buf, None, meta)
    },
    Some(p) => {
      let path = PathBuf::from(p);
      let src = std::fs::read_to_string(&path)?;
      let dir = path.parent().map(|p| p.to_path_buf());
      let label = path.file_name().unwrap().to_string_lossy().into_owned();
      let meta = Arc::new(SourceMeta {
        path: Arc::from(label.clone()),
        version: 0,
        origin: Origin::File(path),
      });
      (label, src, dir, meta)
    },
  };

  // Single diagnostic engine threaded through every phase.
  let mut diag = DiagnosticEngine::new();

  // Lex + parse.
  let mut lexer = Lexer::new(&source, meta.clone(), &mut diag);
  let _ = lexer.scan_tokens();
  let tokens = std::mem::take(&mut lexer.tokens);
  drop(lexer);
  let mut doc = {
    let mut parser = Parser::new(tokens.clone(), meta.clone(), &mut diag);
    parser.parse()
  };

  // Snapshot pre-transform AST for JSON emission / before-tree print.
  let pre_doc = doc.clone();

  // Build pipeline. `code-import` resolves `file=` paths relative to the
  // input mdx's parent dir so samples co-located with their snippets work.
  let applied: Vec<String> = match passes {
    Some(ref names) => {
      names.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
    },
    None => vec!["bare-url".into(), "autolink-headings".into()],
  };
  let pipeline = match passes {
    Some(ref names) => build_pipeline_from_names(names),
    None => Pipeline::with_defaults(),
  };

  // Run transforms.
  pipeline.run(&mut doc, &meta, &mut diag);

  let errors = diag.error_count();
  let warnings = diag.warning_count();
  let total = diag.iter().count();

  // JSON mode → full structured dump, exit early.
  if show_json {
    let want_pre = diff || (!after_only);
    let want_post = !before_only;
    let mut out = json!({
      "label": label,
      "passes": applied,
      "topLevelNodes": doc.children.len(),
      "tokenCount": tokens.len(),
      "errors": errors,
      "warnings": warnings,
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
    if want_pre {
      out["before"] = serde_json::to_value(&pre_doc).unwrap_or(serde_json::Value::Null);
    }
    if want_post {
      out["after"] = serde_json::to_value(&doc).unwrap_or(serde_json::Value::Null);
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
      println!(
        "  [{:>4}] {:?} @ {}:{} len={}  {:?}",
        i, t.kind, t.span.line, t.span.column, t.span.length, raw
      );
    }
  }

  let want_pre = diff || (!after_only);
  let want_post = !before_only;
  if want_pre {
    if !tree_only {
      println!("\n-- ast (pre-transform) --");
    }
    print_doc(&pre_doc);
  }
  if want_post {
    if !tree_only {
      println!("\n-- ast (post-transform) --");
    }
    print_doc(&doc);
  }

  if tree_only {
    return Ok(());
  }

  if !quiet && total > 0 {
    let color = std::io::IsTerminal::is_terminal(&std::io::stdout());
    println!("\n-- diagnostics ({}) --", total);
    print!("{}", duck_diagnostic::format_all_smart(&diag, color));
  }

  if show_debug {
    println!("\n-- debug --\n{:#?}", doc);
  }

  if !quiet {
    println!("\nsummary: {} top-level node(s), {} diagnostic(s)", doc.children.len(), total);
  }
  Ok(())
}

fn print_pass_list() {
  let names = [
    ("autolink-headings", "Wrap heading children in `<a href=\"#id\">`."),
    ("bare-url", "Convert bare http(s) URLs in paragraphs into Link nodes."),
    ("code-import", "Replace `file=\"...\"` code-block meta with file contents."),
    ("component-source", "Replace `<ComponentSource path=\"...\" />` with a code block."),
    (
      "component-preview",
      "Replace `<ComponentPreview name=\"...\" />` with first file's contents.",
    ),
    ("disable-gfm", "Strip GFM-only constructs (tables, strikethrough, task lists)."),
  ];
  println!("available transformers:");
  for (n, d) in names {
    println!("  {:<20}  {}", n, d);
  }
  println!("\npass --passes <comma,separated,names> to pick a subset.");
  println!("default pipeline (no --passes): code-import, bare-url, autolink-headings.");
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
      "mermaid" => p.add(dmc_transform::Mermaid::default()),
      "npm-command" => p.add(dmc_transform::NpmCommand),
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
