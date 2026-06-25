//! Printer 单元测试。

use super::*;
use crate::{DomRegistry, NodeId, VNode, VElementVNode};

#[test]
fn test_print_empty_tree() {
    let doc = DomRegistry::new();
    // 无节点时打印不应 panic
    let output = print_dom_tree(&doc, NodeId::new(0));
    assert!(output.contains("DOM Tree"));
    assert!(output.contains("Style Dirty Set"));
    assert!(output.contains("Computed Styles"));
}

#[test]
fn test_print_simple_tree() {
    let mut doc = DomRegistry::new();
    let vnode = VNode::Element(VElementVNode {
        tagName: "div".into(),
        id: Some("root".into()),
        childNodes: vec![VNode::Text("hello".into())],
        ..Default::default()
    });
    let root = doc.vnodeToDom(&vnode);

    let output = print_dom_tree(&doc, root);
    assert!(output.contains("div#root"));
    assert!(output.contains("hello"));
    assert!(output.contains("Style Dirty Set"));
    assert!(output.contains("Computed Styles"));
}

#[test]
fn test_print_with_styles() {
    let mut doc = DomRegistry::new();
    let vnode = VNode::Element(VElementVNode {
        tagName: "button".into(),
        classList: vec!["btn".into()],
        ..Default::default()
    });
    let root = doc.vnodeToDom(&vnode);

    // 模拟设置计算样式
    let mut style = crate::ComputedStyle::default();
    style.bgColor = 0xFF007BFFu32;
    style.fontSize = 16.0;
    doc.allNodes.get_mut(&root).unwrap().computedStyle = style;

    let output = print_dom_tree(&doc, root);
    assert!(output.contains("button.btn"));
    assert!(output.contains("bg="));
    assert!(output.contains("fs=16"));
}
