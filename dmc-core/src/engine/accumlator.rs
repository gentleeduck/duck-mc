use dmc_codegen::{NodeSink, WalkCtx};
use dmc_parser::ast::Node;

use crate::engine::compile::{CompileConfig, CompileOutput, Metadata, TocItem};

#[derive(Debug)]
pub struct Accumulator {
  // collected during walk
  pub frontmatter: serde_json::Value,
  pub frontmatter_raw: String,
  pub imports: Vec<String>,
  pub exports: Vec<String>,
  pub plain: String,                       // text for excerpt + word count
  pub toc_flat: Vec<(u8, String, String)>, // (level, title, slug) pre-nest

  // transient state during heading capture
  in_heading: Option<(u8, String)>,
  heading_text: String,
}

impl NodeSink for Accumulator {
  fn enter(&mut self, node: &Node, _ctx: &WalkCtx) {
    match node {
      Node::Frontmatter(f) => {
        self.frontmatter_raw = f.raw.clone();
        self.frontmatter = serde_yaml::from_str(&f.raw).unwrap_or(serde_json::Value::Null);
      },
      Node::Import(i) => self.imports.push(i.raw.clone()),
      Node::Export(x) => self.exports.push(x.raw.clone()),
      Node::Heading(h) => {
        self.in_heading = Some((h.level, h.slug()));
        self.heading_text.clear();
      },
      Node::Text(t) => {
        if self.in_heading.is_some() {
          self.heading_text.push_str(&t.value);
        }
        self.plain.push_str(&t.value)
      },
      Node::InlineCode(c) => {
        if self.in_heading.is_some() {
          self.heading_text.push_str(&c.value);
        }
        self.plain.push_str(&c.value);
      },
      Node::CodeBlock(cb) => {
        if self.in_heading.is_some() {
          self.heading_text.push_str(&cb.value);
        }
        self.plain.push_str(&cb.value);
      },
      Node::Image(i) => self.plain.push_str(&i.alt),
      _ => {},
    }
  }
  fn leave(&mut self, node: &Node, _ctx: &WalkCtx) {
    match node {
      Node::Heading(_) => {
        if let Some((level, slug)) = self.in_heading.take() {
          self.toc_flat.push((level, std::mem::take(&mut self.heading_text).trim().to_string(), slug));
        }
      },
      Node::Paragraph(_) => self.plain.push('\n'),
      _ => {},
    }
  }
}

impl Accumulator {
  pub fn new() -> Self {
    Self {
      frontmatter: serde_json::Value::Null,
      frontmatter_raw: String::new(),
      imports: Vec::new(),
      exports: Vec::new(),
      plain: String::new(),
      toc_flat: Vec::new(),
      in_heading: None,
      heading_text: String::new(),
    }
  }

  /// Consume self + the other sinks' rendered outputs; assemble the
  /// final `CompileOutput`. `cfg` is reserved for future excerpt-length /
  /// reading-rate tuning; currently unused.
  pub fn into_compile_output(
    self,
    source: &str,
    html: String,
    body: String,
    _cfg: &CompileConfig,
  ) -> CompileOutput {
    let content = Self::frontmatter(source).to_string();
    let excerpt = Self::excerpt(&self.plain, 260);
    let metadata = Self::metadata(&self.plain);
    let toc = Self::toc(&self.toc_flat);

    CompileOutput {
      frontmatter: self.frontmatter,
      frontmatter_raw: self.frontmatter_raw,
      content,
      html,
      body,
      excerpt,
      metadata,
      toc,
      imports: self.imports,
      exports: self.exports,
    }
  }

  /// `source` minus a leading `---...---` YAML frontmatter block (BOM tolerant).
  fn frontmatter(source: &str) -> &str {
    let s = source.trim_start_matches('\u{feff}');
    if !s.starts_with("---") {
      return source;
    }
    // find the next "---" on its own line
    let after = &s[3..];
    if let Some(end) = after.find("\n---") {
      let rest_start = 3 + end + 4; // 3 dashes + \n--- = 4 chars after end
      // skip optional newline after the closing ---
      let rest = &s[rest_start..];
      let rest = rest.trim_start_matches('\n');
      return rest;
    }
    source
  }

  /// Collapse whitespace, truncate to `max` chars, append `...` if cut.
  /// Char-aware (multibyte safe).
  fn excerpt(plain: &str, max: usize) -> String {
    let s: String = plain.split_whitespace().collect::<Vec<_>>().join(" ");
    if s.chars().count() <= max {
      return s;
    }
    let truncated: String = s.chars().take(max).collect();
    format!("{}...", truncated.trim_end())
  }

  /// Word count + reading time from plain text. 200 wpm, ceil, min 1 min.
  fn metadata(plain: &str) -> Metadata {
    let words = plain.split_whitespace().count() as u32;
    let reading = ((words as f32) / 200.0).ceil() as u32;
    Metadata { word_count: words, reading_time: reading.max(1) }
  }

  /// Flat `(level, title, slug)` list -> hierarchical `TocItem` tree.
  /// Level stack tracks ancestry; new headings nest under the last open
  /// parent or pop back to an earlier ancestor.
  fn toc(items: &[(u8, String, String)]) -> Vec<TocItem> {
    let mut roots: Vec<TocItem> = Vec::new();
    // index path into the children tree, parallel with the level stack
    let mut path: Vec<usize> = Vec::new();
    let mut levels: Vec<u8> = Vec::new();
    for (level, title, id) in items {
      let item = TocItem { title: title.clone(), url: format!("#{}", id), items: Vec::new() };
      // pop until top has lower level
      while let Some(top) = levels.last() {
        if *top >= *level {
          levels.pop();
          path.pop();
        } else {
          break;
        }
      }
      // navigate to insertion list
      let parent_list: &mut Vec<TocItem> = if path.is_empty() {
        &mut roots
      } else {
        let mut node = &mut roots[path[0]];
        for idx in &path[1..] {
          node = &mut node.items[*idx];
        }
        &mut node.items
      };
      parent_list.push(item);
      let new_idx = parent_list.len() - 1;
      path.push(new_idx);
      levels.push(*level);
    }
    roots
  }
}
