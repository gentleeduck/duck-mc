//! `<ComponentSource>` resolver. See `transformers/component-source.md`
//! for full docs.

use crate::pipeline::Transformer;
use crate::visit::{NodeAction, Visitor, walk_root};
use dmc_diagnostic::metadata::{Origin, SourceMeta};
use dmc_diagnostic::{Code, DiagResult};
use dmc_parser::ast::*;
use duck_diagnostic::{Diagnostic, Label, diag};
use std::path::PathBuf;
use std::sync::Arc;

/// Resolve `<ComponentSource path="..." />` JSX nodes by reading the
/// referenced file (or directory of files) and injecting one
/// `CodeBlock` child per file. The JSX wrapper stays so consumers can
/// render Preview/Code chrome around the resolved source. PrettyCode
/// then highlights every injected `CodeBlock` natively.
#[derive(Default)]
pub struct ComponentSource {
  pub base_dir: Option<PathBuf>,
}

impl ComponentSource {
  pub fn with_base_dir(p: impl Into<PathBuf>) -> Self {
    Self { base_dir: Some(p.into()) }
  }

  fn attr_value(attrs: &[JsxAttr], name: &str) -> Option<String> {
    attrs.iter().find(|a| a.name == name).and_then(|a| match &a.value {
      JsxAttrValue::String(s) => Some(s.clone()),
      _ => None,
    })
  }
}

impl Transformer for ComponentSource {
  fn name(&self) -> &str {
    "component-source"
  }
  fn transform(
    &self,
    doc: &mut Document,
    meta: &SourceMeta,
    diag_engine: &mut duck_diagnostic::DiagnosticEngine<Code>,
  ) {
    let base_dir = self.base_dir.clone().or_else(|| match &meta.origin {
      Origin::File(p) => p.parent().map(|p| p.to_path_buf()),
      _ => None,
    });

    if base_dir.is_none() && self.base_dir.is_none() {
      diag_engine.emit(diag!(
        Code::BaseDirNotFound,
        format!(
          "component-source: source has no on-disk parent (origin = {:?}); relative `path=` cannot be resolved",
          meta.origin
        )
      ));
    }

    let mut v = Apply { base_dir, meta_path: meta.path.clone(), pending: Vec::new() };
    walk_root(&mut doc.children, &mut v);
    for d in v.pending.drain(..) {
      diag_engine.emit(d);
    }
  }
}

struct Apply {
  base_dir: Option<PathBuf>,
  meta_path: Arc<str>,
  pending: Vec<Diagnostic<Code>>,
}

/// Build a `CodeBlock` node from a single file's content. `rel_label` is
/// the user-visible filename emitted as the block's `title=` meta so the
/// React `<ComponentSource>` wrapper can label tabs by basename.
#[allow(clippy::result_large_err)]
fn make_code_block(abs: &PathBuf, rel_label: &str, span: &duck_diagnostic::Span) -> DiagResult<Node> {
  let content =
    std::fs::read_to_string(abs).map_err(|e| diag!(Code::IoRead, format!("read {}: {}", abs.display(), e)))?;
  let lang = abs.extension().and_then(|s| s.to_str()).map(String::from);
  Ok(Node::CodeBlock(CodeBlock {
    lang,
    meta: Some(format!("title=\"{}\"", rel_label)),
    value: content,
    span: span.clone(),
  }))
}

/// Read a path that may be a single file or a directory of files. For
/// directories, yields one `CodeBlock` per direct child file (sorted).
#[allow(clippy::result_large_err)]
fn collect_blocks(abs: &PathBuf, rel: &str, span: &duck_diagnostic::Span) -> DiagResult<Vec<Node>> {
  let stat = std::fs::metadata(abs).map_err(|e| diag!(Code::IoRead, format!("stat {}: {}", abs.display(), e)))?;
  if stat.is_dir() {
    let mut entries: Vec<_> = std::fs::read_dir(abs)
      .map_err(|e| diag!(Code::IoRead, format!("read_dir {}: {}", abs.display(), e)))?
      .filter_map(|e| e.ok())
      .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
      .collect();
    entries.sort_by_key(|e| e.file_name());
    let mut blocks = Vec::with_capacity(entries.len());
    for e in entries {
      let path = e.path();
      let label = e.file_name().to_string_lossy().into_owned();
      blocks.push(make_code_block(&path, &label, span)?);
    }
    Ok(blocks)
  } else {
    let label = abs.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| rel.to_string());
    Ok(vec![make_code_block(abs, &label, span)?])
  }
}

impl Visitor for Apply {
  fn visit_node(&mut self, node: &mut Node) -> NodeAction {
    let (path_attr, attrs, span, was_self_closing) = match node {
      Node::JsxSelfClosing(j) if j.name == "ComponentSource" => {
        (ComponentSource::attr_value(&j.attrs, "path"), std::mem::take(&mut j.attrs), j.span.clone(), true)
      },
      Node::JsxElement(j) if j.name == "ComponentSource" => {
        (ComponentSource::attr_value(&j.attrs, "path"), std::mem::take(&mut j.attrs), j.span.clone(), false)
      },
      _ => return NodeAction::Keep,
    };

    let Some(rel) = path_attr else {
      self.pending.push(
        diag!(Code::MissingComponentAttr, "component-source: missing required `path` attribute".to_string())
          .with_label(Label::primary(span, Some("on this <ComponentSource>".into()))),
      );
      // Restore attrs since we took them
      if let Node::JsxSelfClosing(j) = node {
        j.attrs = attrs;
      } else if let Node::JsxElement(j) = node {
        j.attrs = attrs;
      }
      return NodeAction::Keep;
    };

    let abs = match &self.base_dir {
      Some(b) => b.join(&rel),
      None => PathBuf::from(&rel),
    };
    let children = match collect_blocks(&abs, &rel, &span) {
      Ok(bs) => bs,
      Err(e) => {
        self.pending.push(
          diag!(
            Code::ComponentSourceUnreadable,
            format!("component-source: cannot read {} ({})", abs.display(), e.message)
          )
          .with_label(Label::primary(span.clone(), Some(format!("from {}", self.meta_path)))),
        );
        if was_self_closing {
          if let Node::JsxSelfClosing(j) = node {
            j.attrs = attrs;
          }
        } else if let Node::JsxElement(j) = node {
          j.attrs = attrs;
        }
        return NodeAction::Keep;
      },
    };

    // Replace self-closing / empty-element with a populated JsxElement
    // so the React `<ComponentSource>` wrapper sees the resolved code
    // blocks as `children`. Pretty-code highlights them on its turn.
    *node = Node::JsxElement(JsxElement { name: "ComponentSource".into(), attrs, children, span });
    NodeAction::KeepSkipChildren
  }
}
