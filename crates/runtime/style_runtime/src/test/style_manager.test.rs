//! 样式引擎集成测试。

use super::*;
use dom_flat::*;

#[test]
fn test_parse_simple_css() {
    let css = r"
        body { margin: 0; padding: 0; }
        .container { background: #f5f5f5; }
    ";
    let rules = parse_css(css);
    assert_eq!(rules.len(), 2);
    assert_eq!(rules[0].style.len(), 2);
    assert_eq!(rules[1].style.len(), 1);
}

#[test]
fn test_parse_css_with_comments() {
    let css = r"
        /* 注释 */
        body { margin: 0; }
        /* 另一条注释 */
        h1 { font-size: 24px; }
    ";
    let rules = parse_css(css);
    assert_eq!(rules.len(), 2);
}

#[test]
fn test_full_flow_with_structured_rules() {
    let mut doc = DomRegistry::new();
    let mut engine = StyleEngine::new();

    let vnode = VNode::Element(VElementVNode {
        tagName: "div".into(),
        classList: vec!["container".into()],
        childNodes: vec![
            VNode::Element(VElementVNode {
                tagName: "h1".into(),
                childNodes: vec![VNode::Text("Title".into())],
                ..Default::default()
            }),
            VNode::Element(VElementVNode {
                tagName: "button".into(),
                id: Some("btn".into()),
                classList: vec!["primary".into()],
                childNodes: vec![VNode::Text("Click".into())],
                ..Default::default()
            }),
        ],
        ..Default::default()
    });

    let root_id = doc.vnodeToDom(&vnode);

    // 创建样式表并加入 document.styleSheets
    let sheet = CssStyleSheet {
        sheetType: "text/css".into(),
        href: None,
        title: None,
        media: "all".into(),
        disabled: false,
        cssRules: vec![
            CssRule::on(Select::class("container"))
                .decl("background", "#f5f5f5")
                .decl("padding", "20px"),
            CssRule::on(Select::tag("h1"))
                .decl("font-size", "24px")
                .decl("color", "#333333"),
            CssRule::on(Select::id("btn"))
                .decl("color", "white"),
            CssRule::on(Select::class("primary"))
                .decl("background", "#007bff")
                .decl("font-size", "16px"),
        ],
    };
    doc.styleSheets.push(sheet);
    engine.rebuildIndices(&doc.styleSheets);

    doc.markAllDirty(root_id);
    engine.flushStyleDirty(&mut doc);

    let root = doc.allNodes.get(&root_id).unwrap();
    assert!(!root.styleDirty);
    assert!(root.layoutDirty);
    assert_eq!(root.computedStyle.bgColor, 0xFFF5F5F5);
    assert_eq!(root.computedStyle.padding, 20.0);

    let h1_id = root.childNodes[0];
    let h1 = doc.allNodes.get(&h1_id).unwrap();
    assert_eq!(h1.computedStyle.fontSize, 24.0);
    assert_eq!(h1.computedStyle.textColor, 0xFF333333);

    let btn_id = root.childNodes[1];
    let btn = doc.allNodes.get(&btn_id).unwrap();
    assert_eq!(btn.computedStyle.textColor, 0xFFFFFFFF);
    assert_eq!(btn.computedStyle.bgColor, 0xFF007BFF);
    assert_eq!(btn.computedStyle.fontSize, 16.0);
}

#[test]
fn test_inline_style_priority() {
    let mut doc = DomRegistry::new();
    let mut engine = StyleEngine::new();

    let vnode = VNode::Element(VElementVNode {
        tagName: "div".into(),
        classList: vec!["box".into()],
        style: "padding: 50px; margin: 10px".into(),
        ..Default::default()
    });
    let root_id = doc.vnodeToDom(&vnode);

    let sheet = CssStyleSheet {
        sheetType: "text/css".into(),
        href: None,
        title: None,
        media: "all".into(),
        disabled: false,
        cssRules: vec![
            CssRule::on(Select::class("box"))
                .decl("padding", "10px")
                .decl("margin", "0")
                .decl("font-size", "14px"),
        ],
    };
    doc.styleSheets.push(sheet);
    engine.rebuildIndices(&doc.styleSheets);

    doc.markAllDirty(root_id);
    engine.flushStyleDirty(&mut doc);

    let node = doc.allNodes.get(&root_id).unwrap();
    assert_eq!(node.computedStyle.padding, 50.0);
    assert_eq!(node.computedStyle.margin, 10.0);
    assert_eq!(node.computedStyle.fontSize, 14.0);
}
