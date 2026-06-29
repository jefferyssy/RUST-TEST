//! DomRegistry 单元测试。

use super::*;
use crate::{VNode, VElementVNode};

#[test]
fn test_create_element() {
    let mut doc = DomRegistry::new();
    let div = doc.createElement("div");
    assert_eq!(div, NodeId::new(0));
    assert_eq!(doc.allNodes.len(), 1);
    assert!(doc.tagMap.contains_key("div"));
    assert_eq!(doc.tagMap["div"], vec![NodeId::new(0)]);
}

#[test]
fn test_create_text_node() {
    let mut doc = DomRegistry::new();
    let text = doc.createTextNode("hello");
    assert_eq!(text, NodeId::new(0));
    let node = doc.allNodes.get(&text).unwrap();
    assert!(node.is_text());
    assert_eq!(node.text_content(), Some("hello"));
}

#[test]
fn test_append_child() {
    let mut doc = DomRegistry::new();
    let parent = doc.createElement("div");
    let child = doc.createElement("span");
    doc.appendChild(parent, child);

    let parent_node = doc.allNodes.get(&parent).unwrap();
    assert_eq!(parent_node.childNodes, vec![child]);

    let child_node = doc.allNodes.get(&child).unwrap();
    assert_eq!(child_node.parentNode, Some(parent));
}

#[test]
fn test_set_id_attribute() {
    let mut doc = DomRegistry::new();
    let div = doc.createElement("div");
    doc.setAttribute(div, "id", "app");

    let node = doc.allNodes.get(&div).unwrap();
    assert_eq!(node.id, Some("app".to_string()));
    assert_eq!(doc.getElementById("app"), Some(div));
}

#[test]
fn test_set_class_attribute() {
    let mut doc = DomRegistry::new();
    let div = doc.createElement("div");
    doc.setAttribute(div, "class", "foo bar");

    let node = doc.allNodes.get(&div).unwrap();
    assert_eq!(node.classList, vec!["foo", "bar"]);
    assert_eq!(doc.getElementsByClassName("foo"), vec![div]);
    assert_eq!(doc.getElementsByClassName("bar"), vec![div]);
}

#[test]
fn test_remove_child() {
    let mut doc = DomRegistry::new();
    let parent = doc.createElement("div");
    let child = doc.createElement("span");
    doc.appendChild(parent, child);
    doc.removeChild(parent, child);

    let parent_node = doc.allNodes.get(&parent).unwrap();
    assert!(parent_node.childNodes.is_empty());
    assert!(!doc.allNodes.contains_key(&child));
}

#[test]
fn test_mark_all_dirty() {
    let mut doc = DomRegistry::new();
    let root = doc.createElement("div");
    let child = doc.createElement("span");
    doc.appendChild(root, child);

    doc.allNodes.get_mut(&root).unwrap().styleDirty = false;
    doc.allNodes.get_mut(&child).unwrap().styleDirty = false;
    doc.styleDirtySet.clear();

    doc.markAllDirty(root);
    assert!(doc.allNodes.get(&root).unwrap().styleDirty);
    assert!(doc.allNodes.get(&child).unwrap().styleDirty);
    assert_eq!(doc.styleDirtySet.len(), 2);
}

#[test]
fn test_get_elements_by_tag() {
    let mut doc = DomRegistry::new();
    let div = doc.createElement("div");
    let span = doc.createElement("span");
    assert_eq!(doc.getElementsByTagName("div"), vec![div]);
    assert_eq!(doc.getElementsByTagName("span"), vec![span]);
    assert!(doc.getElementsByTagName("p").is_empty());
}

#[test]
fn test_set_text_content() {
    let mut doc = DomRegistry::new();
    let div = doc.createElement("div");
    doc.setTextContent(div, "hello world");

    let node = doc.allNodes.get(&div).unwrap();
    assert_eq!(node.childNodes.len(), 1);
    let text_id = node.childNodes[0];
    let text_node = doc.allNodes.get(&text_id).unwrap();
    assert!(text_node.is_text());
    assert_eq!(text_node.text_content(), Some("hello world"));
}

#[test]
fn test_style_dirty_on_set_attribute() {
    let mut doc = DomRegistry::new();
    let div = doc.createElement("div");
    // 清除初始 dirty
    doc.allNodes.get_mut(&div).unwrap().styleDirty = false;
    doc.styleDirtySet.clear();

    doc.setAttribute(div, "class", "new-class");

    let node = doc.allNodes.get(&div).unwrap();
    assert!(node.styleDirty);
    assert!(doc.styleDirtySet.contains(&div));
}

#[test]
fn test_query_selector() {
    let mut doc = DomRegistry::new();
    let root = doc.createElement("div");
    doc.setAttribute(root, "id", "root");

    let child = doc.createElement("span");
    doc.setAttribute(child, "class", "highlight");
    doc.appendChild(root, child);

    // vnodeToDom 构建 body → 然后 querySelector
    // 先手动设 root 为 body（无 parent）
    // querySelector 从无 parent 的元素开始
    assert_eq!(doc.querySelector("#root"), Some(root));
    assert_eq!(doc.querySelector(".highlight"), Some(child));
    assert!(doc.querySelector(".nonexistent").is_none());
}

#[test]
fn test_vnode_to_dom() {
    let mut doc = DomRegistry::new();
    let vnode = VNode::Element(VElementVNode {
        tagName: "div".into(),
        id: Some("app".into()),
        classList: vec!["container".into()],
        childNodes: vec![
            VNode::Text("Hello".into()),
            VNode::Element(VElementVNode {
                tagName: "span".into(),
                classList: vec!["item".into()],
                ..Default::default()
            }),
        ],
        ..Default::default()
    });

    let root_id = doc.vnodeToDom(&vnode);

    // 验证根节点
    let root = doc.allNodes.get(&root_id).unwrap();
    assert_eq!(root.tagName.as_deref(), Some("div"));
    assert_eq!(root.id.as_deref(), Some("app"));
    assert_eq!(root.classList, vec!["container"]);
    assert_eq!(root.childNodes.len(), 2);

    // 验证文本子节点
    let text_id = root.childNodes[0];
    let text_node = doc.allNodes.get(&text_id).unwrap();
    assert!(text_node.is_text());
    assert_eq!(text_node.text_content(), Some("Hello"));

    // 验证元素子节点
    let span_id = root.childNodes[1];
    let span = doc.allNodes.get(&span_id).unwrap();
    assert_eq!(span.tagName.as_deref(), Some("span"));
    assert_eq!(span.classList, vec!["item"]);

    // 验证索引
    assert_eq!(doc.getElementById("app"), Some(root_id));
    assert_eq!(doc.getElementsByClassName("container"), vec![root_id]);
    assert_eq!(doc.getElementsByClassName("item"), vec![span_id]);
}
