use dmc_codegen::{NodeSink, WalkCtx};
use dmc_parser::ast::Node;

use crate::engine::compile::{CompileConfig, CompileOutput, Metadata, TocItem};

#[derive(Debug)]
pub struct Accumulator {
  pub frontmatter: serde_json::Value,
  pub frontmatter_raw: String,
  pub imports: Vec<String>,
  pub exports: Vec<String>,
  /// Text for excerpt + word count.
  pub plain: String,
  /// `(level, title, slug)` pre-nest.
  pub toc_flat: Vec<(u8, String, String)>,

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

impl Default for Accumulator {
  fn default() -> Self {
    Self::new()
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

  /// `cfg` reserved for future excerpt-length / reading-rate tuning.
  pub fn into_compile_output(self, source: &str, html: String, body: String, _cfg: &CompileConfig) -> CompileOutput {
    let content = Self::frontmatter(source).to_string();
    let excerpt = Self::excerpt(&self.plain, 260);
    // Match velite: `wordCount` from the markdown body (captures
    // structural words + JSX text the AST walker drops); `readingTime`
    // from plain prose (no source noise).
    let metadata = Self::metadata(&content, &self.plain);
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
    let after = &s[3..];
    if let Some(end) = after.find("\n---") {
      let rest_start = 3 + end + 4;
      let rest = &s[rest_start..];
      let rest = rest.trim_start_matches('\n');
      return rest;
    }
    source
  }

  /// Collapse whitespace, truncate to `max` chars (char-aware), append `...` if cut.
  fn excerpt(plain: &str, max: usize) -> String {
    let s: String = plain.split_whitespace().collect::<Vec<_>>().join(" ");
    if s.chars().count() <= max {
      return s;
    }
    let truncated: String = s.chars().take(max).collect();
    format!("{}...", truncated.trim_end())
  }

  /// Strips fenced code blocks and ATX heading markers (the `#`-runs
  /// would otherwise count as words and overshoot velite by ~50/doc).
  /// 200 wpm, half-up rounding, min 1 min.
  fn metadata(source: &str, plain: &str) -> Metadata {
    let mut filtered = String::with_capacity(source.len());
    let mut in_fence = false;
    for line in source.lines() {
      if line.trim_start().starts_with("```") {
        in_fence = !in_fence;
        continue;
      }
      if in_fence {
        continue;
      }
      // Drop the leading `#`-run so `## 0.4.3` counts as one word.
      let trimmed = line.trim_start();
      if let Some(rest) = trimmed.strip_prefix(|c: char| c == '#') {
        let mut after_hashes = rest;
        while let Some(r) = after_hashes.strip_prefix('#') {
          after_hashes = r;
        }
        if after_hashes.starts_with(' ') || after_hashes.starts_with('\t') || after_hashes.is_empty() {
          filtered.push_str(after_hashes);
          filtered.push('\n');
          continue;
        }
      }
      filtered.push_str(line);
      filtered.push('\n');
    }
    let words = filtered.split_whitespace().count() as u32;
    let plain_words = plain.split_whitespace().count() as u32;
    // Match velite's `Math.round` (half-up) for reading-time.
    let reading = ((plain_words as f32) / 200.0).round() as u32;
    Metadata { word_count: words, reading_time: reading.max(1) }
  }

  /// Flat `(level, title, slug)` list -> hierarchical `TocItem` tree.
  fn toc(items: &[(u8, String, String)]) -> Vec<TocItem> {
    let mut roots: Vec<TocItem> = Vec::new();
    let mut path: Vec<usize> = Vec::new();
    let mut levels: Vec<u8> = Vec::new();
    for (level, title, id) in items {
      let item = TocItem { title: title.clone(), url: format!("#{}", id), items: Vec::new() };
      while let Some(top) = levels.last() {
        if *top >= *level {
          levels.pop();
          path.pop();
        } else {
          break;
        }
      }
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
