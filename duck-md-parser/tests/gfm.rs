mod common;
use common::*;
use duck_md_ast::*;
use pretty_assertions::assert_eq;

fn first_paragraph(d: &Document) -> &Paragraph {
    for c in &d.children {
        if let Node::Paragraph(p) = c {
            return p;
        }
    }
    panic!("no paragraph");
}

#[test]
fn parses_strikethrough() {
    let d = parse_doc("~~bye~~");
    let p = first_paragraph(&d);
    assert!(
        p.children.iter().any(|n| matches!(n, Node::Strikethrough(_))),
        "got {:?}",
        p.children
    );
}

#[test]
fn parses_task_list() {
    let src = "- [ ] one\n- [x] two\n";
    let d = parse_doc(src);
    let l = d
        .children
        .iter()
        .find_map(|n| match n {
            Node::List(l) => Some(l),
            _ => None,
        })
        .expect("list");
    assert_eq!(l.children.len(), 2);
    let mut tasks = 0;
    let mut checked_count = 0;
    for c in &l.children {
        if let Node::TaskListItem(t) = c {
            tasks += 1;
            if t.checked {
                checked_count += 1;
            }
        }
    }
    assert_eq!(tasks, 2);
    assert_eq!(checked_count, 1);
}
