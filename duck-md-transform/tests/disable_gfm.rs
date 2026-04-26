use duck_md_parser::ast::*;
use duck_md_parser::parse;
use duck_md_transform::{DisableGfm, Pipeline, Transformer};

fn run(src: &str) -> Document {
  let mut doc = parse(src);
  let p = Pipeline::new().add(DisableGfm);
  p.run(&mut doc);
  doc
}

#[test]
fn strikethrough_downgrades_to_text() {
  let doc = run("hello ~~old~~ new\n");
  let para = doc
    .children
    .iter()
    .find_map(|n| match n {
      Node::Paragraph(p) => Some(p),
      _ => None,
    })
    .unwrap();
  let has_strike = para.children.iter().any(|n| matches!(n, Node::Strikethrough(_)));
  assert!(!has_strike, "strikethrough should be downgraded");
  let texts: Vec<&str> = para
    .children
    .iter()
    .filter_map(|n| match n {
      Node::Text(t) => Some(t.value.as_str()),
      _ => None,
    })
    .collect();
  assert!(texts.iter().any(|s| s.contains("~~old~~")), "raw ~~ markers should appear in output");
}

#[test]
fn table_downgrades_to_paragraph() {
  let doc = run("| a | b |\n|---|---|\n| 1 | 2 |\n");
  let has_table = doc.children.iter().any(|n| matches!(n, Node::Table(_)));
  assert!(!has_table, "table should be downgraded");
}

#[test]
fn task_list_loses_checked_marker_node() {
  let doc = run("- [x] done\n- [ ] open\n");
  fn has_task_item(nodes: &[Node]) -> bool {
    nodes.iter().any(|n| {
      matches!(n, Node::TaskListItem(_))
        || (if let Node::List(l) = n { has_task_item(&l.children) } else { false })
        || (if let Node::ListItem(li) = n { has_task_item(&li.children) } else { false })
    })
  }
  assert!(!has_task_item(&doc.children), "TaskListItem should be downgraded to ListItem");
}
