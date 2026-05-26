//! 诊断测试：模拟 todo_app 输入场景，对比键入前后布局
//!
//! 此测试验证：在 input 中输入文本不会导致 footer 位置下移。

use super::*;
use crate::layout_box::LayoutBox;
use dom::Document;
use dom::Size;
use style::cascade::{compute_element_style_with_node, ComputedStyle};
use style::stylesheet::{self, StyleSheet};
use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;
use dom::node::{DirtyType, Node, NodeType};

/// 辅助：递归计算所有 DOM 节点的样式
fn compute_styles_recursive(
    node: &Rc<RefCell<Node>>,
    sheets: &[StyleSheet],
    parent_style: Option<&ComputedStyle>,
    out: &mut HashMap<usize, ComputedStyle>,
) {
    // 从元素属性中提取内联样式
    let inline_decls: Vec<style::stylesheet::Declaration> = {
        let node_ref = node.borrow();
        if let NodeType::Element(elem) = &node_ref.node_type {
            elem.get_attribute("style")
                .map(|s| style::stylesheet::parse_inline_style(&s))
                .unwrap_or_default()
        } else {
            vec![]
        }
    };
    let style = compute_element_style_with_node(node, parent_style, sheets, &inline_decls);
    let ptr = Rc::as_ptr(node) as usize;
    out.insert(ptr, style);
    for child in &node.borrow().child_nodes() {
        let parent_clone = out.get(&ptr).cloned();
        compute_styles_recursive(child, sheets, parent_clone.as_ref(), out);
    }
}

/// 辅助：在布局树中按 id 属性查找节点
fn find_by_id<'a>(root: &'a LayoutBox, needle_id: &str) -> Option<&'a LayoutBox> {
    if let Some(ref n) = root.node {
        if let NodeType::Element(e) = &n.borrow().node_type {
            if e.id().map(|s| s.as_str()) == Some(needle_id) {
                return Some(root);
            }
        }
    }
    for child in &root.children {
        if let Some(found) = find_by_id(child, needle_id) {
            return Some(found);
        }
    }
    None
}

/// 辅助：在布局树中按 class 属性查找节点
fn find_by_class<'a>(root: &'a LayoutBox, needle_class: &str) -> Option<&'a LayoutBox> {
    if let Some(ref n) = root.node {
        if let NodeType::Element(e) = &n.borrow().node_type {
            let class = e.class_name();
            if class.split_whitespace().any(|c| c == needle_class) {
                return Some(root);
            }
        }
    }
    for child in &root.children {
        if let Some(found) = find_by_class(child, needle_class) {
            return Some(found);
        }
    }
    None
}

/// 辅助：在布局树中查找特定文本内容的节点
fn find_text_node(root: &LayoutBox, needle: &str) -> Option<dom::Rect<f32>> {
    if root.box_type == BoxType::Text {
        let text = root.node.as_ref().map(|n| n.borrow().text_content()).unwrap_or_default();
        if text == needle {
            return Some(root.rect);
        }
    }
    for child in &root.children {
        if let Some(r) = find_text_node(child, needle) {
            return Some(r);
        }
    }
    None
}

fn find_button_node<'a>(root: &'a LayoutBox, child_text: &str) -> Option<&'a LayoutBox> {
    if matches!(root.box_type, BoxType::InlineBlock) {
        let has_text = root.children.iter().any(|c| {
            c.node.as_ref().map(|n| n.borrow().text_content()).unwrap_or_default() == child_text
        });
        if has_text {
            return Some(root);
        }
    }
    for child in &root.children {
        if let Some(r) = find_button_node(child, child_text) {
            return Some(r);
        }
    }
    None
}

#[test]
fn test_todo_app_input_typing_layout_stability() {
    // ===== 构建与 todo_app 完全一致的 DOM =====
    let doc = Document::new();
    let doc_ref = doc.borrow();
    let body = doc_ref.body();
    body.borrow_mut().set_style(
        "font-family: Arial, sans-serif; background-color: #f5f5f5; margin: 0; padding: 20px; display: flex; justify-content: center"
    );

    let app = doc_ref.create_element("div");
    app.borrow_mut().set_attribute("id", "app");
    app.borrow_mut().set_style(
        "background: white; border-radius: 8px; box-shadow: 0 2px 10px rgba(0, 0, 0, 0.1); padding: 24px; width: 400px"
    );

    let h1 = doc_ref.create_element("h1");
    h1.borrow_mut().set_text_content("Todo List");
    h1.borrow_mut().set_style("text-align: center; color: #333; margin: 0 0 16px 0; font-size: 24px");
    app.borrow_mut().append_child(h1.clone());

    let input_group = doc_ref.create_element("div");
    input_group.borrow_mut().set_attribute("class", "input-group");
    input_group.borrow_mut().set_style("display: flex; gap: 8px; margin-bottom: 16px");

    let todo_input = doc_ref.create_element("input");
    todo_input.borrow_mut().set_attribute("id", "todo-input");
    todo_input.borrow_mut().set_style("flex: 1; padding: 8px 12px; border: 1px solid #ddd; border-radius: 4px; font-size: 14px");
    input_group.borrow_mut().append_child(todo_input.clone());

    let add_btn = doc_ref.create_element("button");
    add_btn.borrow_mut().set_attribute("id", "add-btn");
    add_btn.borrow_mut().set_text_content("Add");
    add_btn.borrow_mut().set_style("background: #4a90d9; color: white; border: none; padding: 8px 16px; border-radius: 4px; cursor: pointer; font-size: 14px");
    input_group.borrow_mut().append_child(add_btn.clone());
    app.borrow_mut().append_child(input_group.clone());

    let todo_list = doc_ref.create_element("ul");
    todo_list.borrow_mut().set_attribute("id", "todo-list");
    todo_list.borrow_mut().set_style("list-style: none; padding: 0; margin: 0 0 16px 0; background: greenyellow");
    app.borrow_mut().append_child(todo_list.clone());

    let footer = doc_ref.create_element("div");
    footer.borrow_mut().set_attribute("class", "footer");
    footer.borrow_mut().set_style("display: flex; justify-content: space-between; align-items: center; padding-top: 8px; border-top: 1px solid #eee");

    let todo_count = doc_ref.create_element("span");
    todo_count.borrow_mut().set_attribute("id", "todo-count");
    todo_count.borrow_mut().set_text_content("0 items");
    todo_count.borrow_mut().set_style("font-size: 12px; color: #888");
    footer.borrow_mut().append_child(todo_count.clone());

    let clear_btn = doc_ref.create_element("button");
    clear_btn.borrow_mut().set_attribute("id", "clear-btn");
    clear_btn.borrow_mut().set_text_content("Clear All");
    clear_btn.borrow_mut().set_style("background: transparent; color: #e74c3c; border: 1px solid #e74c3c; padding: 4px 12px; border-radius: 3px; cursor: pointer; font-size: 12px");
    footer.borrow_mut().append_child(clear_btn.clone());
    app.borrow_mut().append_child(footer.clone());

    body.borrow_mut().append_child(app.clone());
    // Document::new() 已自动构建 html > body 结构，无需手动追加
    let html = doc_ref.document_element();
    drop(doc_ref);

    // ===== 第一轮：初始布局（input 无文本） =====
    let mut styles1: HashMap<usize, ComputedStyle> = HashMap::new();
    let sheets: Vec<StyleSheet> = vec![];
    compute_styles_recursive(&html, &sheets, None, &mut styles1);

    let viewport = Size::new(800.0, 600.0);
    let mut engine1 = LayoutEngine::new();

    let mut root1 = build_layout_tree(&html, &styles1, Some(&mut engine1.text_measurer));
    engine1.layout(&mut root1, viewport);

    // 诊断：打印完整布局树结构
    fn dump_tree(node: &LayoutBox, depth: usize) {
        let indent = "  ".repeat(depth);
        let desc = node.node.as_ref().map(|n| {
            let br = n.borrow();
            match &br.node_type {
                NodeType::Element(e) => format!("<{}> id={:?} class={:?}", e.tag_name(), e.id(), e.class_name()),
                NodeType::Text(t) => format!("\"{}\"", &t.data()[..t.data().len().min(30)]),
                _ => format!("{:?}", node.box_type),
            }
        }).unwrap_or_else(|| format!("{:?}", node.box_type));
        eprintln!("[diag tree] {}rect=({:.0},{:.0},{:.0}x{:.0}) {}",
            indent, node.rect.x, node.rect.y, node.rect.width, node.rect.height, desc);
        for child in &node.children {
            dump_tree(child, depth + 1);
        }
    }
    eprintln!("[diag tree] === BEFORE typing layout tree ===");
    dump_tree(&root1, 0);

    let footer1 = find_by_class(&root1, "footer").expect("should find footer");
    let todo_list1 = find_by_id(&root1, "todo-list").expect("should find ul#todo-list");

    let footer_y_before = footer1.rect.y;
    let todo_h_before = todo_list1.rect.height;

    eprintln!(
        "[diag test] BEFORE typing: footer.y={:.1} todo_list.h={:.1}",
        footer_y_before, todo_h_before
    );

    // ===== 第二轮：模拟在 input 中输入文本 "hello" =====
    todo_input.borrow_mut().set_text_content("hello");

    let mut styles2: HashMap<usize, ComputedStyle> = HashMap::new();
    compute_styles_recursive(&html, &sheets, None, &mut styles2);

    let mut engine2 = LayoutEngine::new();
    let mut root2 = build_layout_tree(&html, &styles2, Some(&mut engine2.text_measurer));
    engine2.layout(&mut root2, viewport);

    let footer2 = find_by_class(&root2, "footer").expect("should find footer");
    let todo_list2 = find_by_id(&root2, "todo-list").expect("should find ul#todo-list");

    let footer_y_after = footer2.rect.y;
    let todo_h_after = todo_list2.rect.height;

    eprintln!(
        "[diag test] AFTER typing 'hello': footer.y={:.1} todo_list.h={:.1}",
        footer_y_after, todo_h_after
    );

    // ===== 断言 =====
    // todo-list 高度不应改变（仍然是空的）
    assert!(
        (todo_h_before - todo_h_after).abs() < 0.1,
        "todo-list height should NOT change when typing in input (before={:.1}, after={:.1})",
        todo_h_before, todo_h_after
    );

    // footer 位置不应改变（关键断言！）
    assert!(
        (footer_y_before - footer_y_after).abs() < 0.5,
        "footer position should NOT move when typing in input (before={:.1}, after={:.1})",
        footer_y_before, footer_y_after
    );

    eprintln!(
        "[diag test] PASS: footer position stable at y={:.1}",
        footer_y_after
    );
}

#[test]
fn test_todo_app_clear_all_text_position() {
    let doc = Document::new();
    let doc_ref = doc.borrow();
    let body = doc_ref.body();
    body.borrow_mut().set_style(
        "font-family: Arial, sans-serif; background-color: #f5f5f5; margin: 0; padding: 20px; display: flex; justify-content: center"
    );

    let app = doc_ref.create_element("div");
    app.borrow_mut().set_style("background: white; border-radius: 8px; box-shadow: 0 2px 10px rgba(0, 0, 0, 0.1); padding: 24px; width: 400px");

    let footer = doc_ref.create_element("div");
    footer.borrow_mut().set_style("display: flex; justify-content: space-between; align-items: center; padding-top: 8px; border-top: 1px solid #eee");

    let todo_count = doc_ref.create_element("span");
    todo_count.borrow_mut().set_text_content("0 items");
    todo_count.borrow_mut().set_style("font-size: 12px; color: #888");
    footer.borrow_mut().append_child(todo_count.clone());

    let clear_btn = doc_ref.create_element("button");
    clear_btn.borrow_mut().set_text_content("Clear All");
    clear_btn.borrow_mut().set_style("background: transparent; color: #e74c3c; border: 1px solid #e74c3c; padding: 4px 12px; border-radius: 3px; cursor: pointer; font-size: 12px");
    footer.borrow_mut().append_child(clear_btn.clone());
    app.borrow_mut().append_child(footer.clone());
    body.borrow_mut().append_child(app.clone());
    let html = doc_ref.document_element();
    drop(doc_ref);

    let mut styles: HashMap<usize, ComputedStyle> = HashMap::new();
    let sheets: Vec<StyleSheet> = vec![];
    compute_styles_recursive(&html, &sheets, None, &mut styles);

    let viewport = Size::new(800.0, 600.0);
    let mut engine = LayoutEngine::new();
    let mut root = build_layout_tree(&html, &styles, Some(&mut engine.text_measurer));
    engine.layout(&mut root, viewport);

    let clear_all_text = find_text_node(&root, "Clear All")
        .expect("should find 'Clear All' text node");

    eprintln!(
        "[diag test] 'Clear All' text rect: ({:.0},{:.0},{:.0}x{:.0})",
        clear_all_text.x, clear_all_text.y, clear_all_text.width, clear_all_text.height
    );

    // 诊断：找到按钮自身的 rect
    let clear_btn_box = find_button_node(&root, "Clear All")
        .expect("should find Clear All button");
    eprintln!(
        "[diag test] 'Clear All' button rect: ({:.0},{:.0},{:.0}x{:.0}) padding=({:.0},{:.0},{:.0},{:.0}) border=({:.0},{:.0},{:.0},{:.0}) content_area=({:.0}x{:.0})",
        clear_btn_box.rect.x, clear_btn_box.rect.y,
        clear_btn_box.rect.width, clear_btn_box.rect.height,
        clear_btn_box.padding.top, clear_btn_box.padding.right,
        clear_btn_box.padding.bottom, clear_btn_box.padding.left,
        clear_btn_box.border.top, clear_btn_box.border.right,
        clear_btn_box.border.bottom, clear_btn_box.border.left,
        clear_btn_box.content_area().width, clear_btn_box.content_area().height,
    );

    assert!(
        clear_all_text.x > 10.0,
        "'Clear All' text x={:.0} should be > 10 (not at left edge)", clear_all_text.x
    );
    assert!(
        clear_all_text.y > 10.0,
        "'Clear All' text y={:.0} should be > 10 (not at top edge)", clear_all_text.y
    );
    // 文本应在按钮内容区域内
    let btn_content_left = clear_btn_box.rect.x + clear_btn_box.padding.left + clear_btn_box.border.left;
    let btn_content_right = clear_btn_box.rect.x + clear_btn_box.rect.width
        - clear_btn_box.padding.right - clear_btn_box.border.right;
    assert!(
        clear_all_text.x >= btn_content_left - 1.0,
        "text x={:.0} should be >= button content left={:.0}", clear_all_text.x, btn_content_left
    );
    assert!(
        (clear_all_text.x + clear_all_text.width) <= btn_content_right + 1.0,
        "text right edge={:.0} should be <= button content right={:.0}",
        clear_all_text.x + clear_all_text.width, btn_content_right
    );
}

/// 测试 partial_layout 路径：模拟 input 键入时，已有 li 的 todo_list 场景
///
/// 用户报告：ul 中 li 越多，在 input 中打字时 footer 下移越多。
/// 此测试验证 partial_layout 后 footer 位置保持稳定。
#[test]
fn test_partial_layout_input_typing_with_li_children() {
    // ===== 构建 DOM（含 3 个 li 子节点在 todo_list 中） =====
    let doc = Document::new();
    let doc_ref = doc.borrow();
    let body = doc_ref.body();
    body.borrow_mut().set_style(
        "font-family: Arial, sans-serif; background-color: #f5f5f5; margin: 0; padding: 20px; display: flex; justify-content: center"
    );

    let app = doc_ref.create_element("div");
    app.borrow_mut().set_attribute("id", "app");
    app.borrow_mut().set_style(
        "background: white; border-radius: 8px; box-shadow: 0 2px 10px rgba(0, 0, 0, 0.1); padding: 24px; width: 400px"
    );

    let h1 = doc_ref.create_element("h1");
    h1.borrow_mut().set_text_content("Todo List");
    h1.borrow_mut().set_style("text-align: center; color: #333; margin: 0 0 16px 0; font-size: 24px");
    app.borrow_mut().append_child(h1.clone());

    let input_group = doc_ref.create_element("div");
    input_group.borrow_mut().set_attribute("class", "input-group");
    input_group.borrow_mut().set_style("display: flex; gap: 8px; margin-bottom: 16px");

    let todo_input = doc_ref.create_element("input");
    todo_input.borrow_mut().set_attribute("id", "todo-input");
    todo_input.borrow_mut().set_style("flex: 1; padding: 8px 12px; border: 1px solid #ddd; border-radius: 4px; font-size: 14px");
    input_group.borrow_mut().append_child(todo_input.clone());

    let add_btn = doc_ref.create_element("button");
    add_btn.borrow_mut().set_text_content("Add");
    add_btn.borrow_mut().set_style("background: #4a90d9; color: white; border: none; padding: 8px 16px; border-radius: 4px; cursor: pointer; font-size: 14px");
    input_group.borrow_mut().append_child(add_btn.clone());
    app.borrow_mut().append_child(input_group.clone());

    let todo_list = doc_ref.create_element("ul");
    todo_list.borrow_mut().set_attribute("id", "todo-list");
    todo_list.borrow_mut().set_style("list-style: none; padding: 0; margin: 0 0 16px 0; background: greenyellow");

    // 添加 3 个 li 子节点（模拟已有 todo 项）
    for i in 0..3 {
        let li = doc_ref.create_element("li");
        li.borrow_mut().set_style("display: flex; padding: 8px 0; border-bottom: 1px solid #eee; gap: 8px");
        let span = doc_ref.create_element("span");
        span.borrow_mut().set_style("flex: 1; font-size: 14px");
        span.borrow_mut().set_text_content(&format!("Item {}", i + 1));
        li.borrow_mut().append_child(span.clone());
        let del_btn = doc_ref.create_element("button");
        del_btn.borrow_mut().set_style("background: #e74c3c; color: white; border: none; padding: 4px 8px; border-radius: 3px; font-size: 12px");
        del_btn.borrow_mut().set_text_content("Delete");
        li.borrow_mut().append_child(del_btn.clone());
        todo_list.borrow_mut().append_child(li.clone());
    }
    app.borrow_mut().append_child(todo_list.clone());

    let footer = doc_ref.create_element("div");
    footer.borrow_mut().set_attribute("class", "footer");
    footer.borrow_mut().set_style("display: flex; justify-content: space-between; align-items: center; padding-top: 8px; border-top: 1px solid #eee");

    let todo_count = doc_ref.create_element("span");
    todo_count.borrow_mut().set_text_content("3 items");
    todo_count.borrow_mut().set_style("font-size: 12px; color: #888");
    footer.borrow_mut().append_child(todo_count.clone());

    let clear_btn = doc_ref.create_element("button");
    clear_btn.borrow_mut().set_text_content("Clear All");
    clear_btn.borrow_mut().set_style("background: transparent; color: #e74c3c; border: 1px solid #e74c3c; padding: 4px 12px; border-radius: 3px; font-size: 12px");
    footer.borrow_mut().append_child(clear_btn.clone());
    app.borrow_mut().append_child(footer.clone());

    body.borrow_mut().append_child(app.clone());
    let html = doc_ref.document_element();
    drop(doc_ref);

    // ===== 第一轮：全量初始布局 =====
    let mut styles: HashMap<usize, ComputedStyle> = HashMap::new();
    let sheets: Vec<StyleSheet> = vec![];
    compute_styles_recursive(&html, &sheets, None, &mut styles);

    let viewport = Size::new(800.0, 600.0);
    let mut engine = LayoutEngine::new();

    let mut root = build_layout_tree(&html, &styles, Some(&mut engine.text_measurer));
    engine.layout(&mut root, viewport);

    fn dump_positions(root: &LayoutBox, label: &str) {
        let footer = find_by_class(root, "footer");
        let todo_list = find_by_id(root, "todo-list");
        let input_group = find_by_class(root, "input-group");
        eprintln!(
            "[diag partial] {} footer.y={:.1} todo_list.h={:.1} input_group.h={:.1}",
            label,
            footer.map(|f| f.rect.y).unwrap_or(0.0),
            todo_list.map(|t| t.rect.height).unwrap_or(0.0),
            input_group.map(|ig| ig.rect.height).unwrap_or(0.0),
        );
    }

    dump_positions(&root, "full layout:");

    let footer_y_full = find_by_class(&root, "footer").unwrap().rect.y;
    let todo_h_full = find_by_id(&root, "todo-list").unwrap().rect.height;

    // ===== 第二轮：模拟 partial_layout（input 键入后） =====
    // 标记 input 为 dirty（set_text_content 已做此事）
    todo_input.borrow_mut().set_text_content("hello");

    // 重建布局树（包含新的文本子节点）
    let mut styles2: HashMap<usize, ComputedStyle> = HashMap::new();
    compute_styles_recursive(&html, &sheets, None, &mut styles2);

    let mut root2 = build_layout_tree(&html, &styles2, Some(&mut engine.text_measurer));

    // 收集脏节点并执行增量布局
    let (dirty_nodes, total) = {
        let mut dirty = Vec::new();
        let mut total = 0;
        fn collect(n: &Rc<RefCell<Node>>, dirty: &mut Vec<Rc<RefCell<Node>>>, total: &mut usize) {
            *total += 1;
            if n.borrow().is_dirty() {
                dirty.push(n.clone());
            }
            for child in &n.borrow().child_nodes() {
                collect(child, dirty, total);
            }
        }
        collect(&html, &mut dirty, &mut total);
        (dirty, total)
    };

    eprintln!(
        "[diag partial] dirty={} total={}",
        dirty_nodes.len(), total
    );

    // 使用增量布局
    engine.partial_layout(&mut root2, &dirty_nodes, viewport, false, false);

    dump_positions(&root2, "partial layout:");

    let footer_y_partial = find_by_class(&root2, "footer").unwrap().rect.y;
    let todo_h_partial = find_by_id(&root2, "todo-list").unwrap().rect.height;

    // ===== 断言 =====
    assert!(
        (todo_h_full - todo_h_partial).abs() < 1.0,
        "todo_list height should NOT change: full={:.1} partial={:.1}",
        todo_h_full, todo_h_partial
    );

    assert!(
        (footer_y_full - footer_y_partial).abs() < 1.0,
        "footer y should be STABLE: full={:.1} partial={:.1}",
        footer_y_full, footer_y_partial
    );

    eprintln!(
        "[diag partial] PASS: footer stable at y={:.1} (full={:.1})",
        footer_y_partial, footer_y_full
    );
}

/// ⚠️ 精确模拟运行时行为测试
///
/// 运行时的关键特征:
/// 1. build_layout_tree(&body, ...) — 根节点是 body,不是 html
/// 2. partial_layout 使用 SAME LayoutEngine (不创建新引擎)
/// 3. dirty node 从 html 收集
///
/// 此测试排查 build_layout_tree 根节点差异是否为 bug 根因。
#[test]
fn test_runtime_exact_partial_layout_body_root() {
    // ===== 构建 DOM（与运行时完全一致） =====
    let doc = Document::new();
    let doc_ref = doc.borrow();
    let body = doc_ref.body();
    body.borrow_mut().set_style(
        "font-family: Arial, sans-serif; background-color: #f5f5f5; margin: 0; padding: 20px; display: flex; justify-content: center"
    );

    let app = doc_ref.create_element("div");
    app.borrow_mut().set_attribute("id", "app");
    app.borrow_mut().set_style(
        "background: white; border-radius: 8px; box-shadow: 0 2px 10px rgba(0, 0, 0, 0.1); padding: 24px; width: 400px"
    );

    let h1 = doc_ref.create_element("h1");
    h1.borrow_mut().set_text_content("Todo List");
    h1.borrow_mut().set_style("text-align: center; color: #333; margin: 0 0 16px 0; font-size: 24px");
    app.borrow_mut().append_child(h1.clone());

    let input_group = doc_ref.create_element("div");
    input_group.borrow_mut().set_attribute("class", "input-group");
    input_group.borrow_mut().set_style("display: flex; gap: 8px; margin-bottom: 16px");

    let todo_input = doc_ref.create_element("input");
    todo_input.borrow_mut().set_attribute("id", "todo-input");
    todo_input.borrow_mut().set_style("flex: 1; padding: 8px 12px; border: 1px solid #ddd; border-radius: 4px; font-size: 14px");
    input_group.borrow_mut().append_child(todo_input.clone());

    let add_btn = doc_ref.create_element("button");
    add_btn.borrow_mut().set_text_content("Add");
    add_btn.borrow_mut().set_style("background: #4a90d9; color: white; border: none; padding: 8px 16px; border-radius: 4px; cursor: pointer; font-size: 14px");
    input_group.borrow_mut().append_child(add_btn.clone());
    app.borrow_mut().append_child(input_group.clone());

    let todo_list = doc_ref.create_element("ul");
    todo_list.borrow_mut().set_attribute("id", "todo-list");
    todo_list.borrow_mut().set_style("list-style: none; padding: 0; margin: 0 0 16px 0; background: greenyellow");

    // 添加 3 个 li 子节点
    for i in 0..3 {
        let li = doc_ref.create_element("li");
        li.borrow_mut().set_style("display: flex; padding: 8px 0; border-bottom: 1px solid #eee; gap: 8px");
        let span = doc_ref.create_element("span");
        span.borrow_mut().set_style("flex: 1; font-size: 14px");
        span.borrow_mut().set_text_content(&format!("Item {}", i + 1));
        li.borrow_mut().append_child(span.clone());
        let del_btn = doc_ref.create_element("button");
        del_btn.borrow_mut().set_style("background: #e74c3c; color: white; border: none; padding: 4px 8px; border-radius: 3px; font-size: 12px");
        del_btn.borrow_mut().set_text_content("Delete");
        li.borrow_mut().append_child(del_btn.clone());
        todo_list.borrow_mut().append_child(li.clone());
    }
    app.borrow_mut().append_child(todo_list.clone());

    let footer = doc_ref.create_element("div");
    footer.borrow_mut().set_attribute("class", "footer");
    footer.borrow_mut().set_style("display: flex; justify-content: space-between; align-items: center; padding-top: 8px; border-top: 1px solid #eee");

    let todo_count = doc_ref.create_element("span");
    todo_count.borrow_mut().set_text_content("3 items");
    todo_count.borrow_mut().set_style("font-size: 12px; color: #888");
    footer.borrow_mut().append_child(todo_count.clone());

    let clear_btn = doc_ref.create_element("button");
    clear_btn.borrow_mut().set_text_content("Clear All");
    clear_btn.borrow_mut().set_style("background: transparent; color: #e74c3c; border: 1px solid #e74c3c; padding: 4px 12px; border-radius: 3px; font-size: 12px");
    footer.borrow_mut().append_child(clear_btn.clone());
    app.borrow_mut().append_child(footer.clone());

    body.borrow_mut().append_child(app.clone());
    let html = doc_ref.document_element();
    drop(doc_ref);

    let viewport = Size::new(800.0, 600.0);
    let sheets: Vec<StyleSheet> = vec![];

    // ===== 第一轮：全量布局（模拟运行时 init） =====
    // ★ 关键：使用 &body 作为根节点，与运行时 window.rs 第 508 行一致
    let mut styles1: HashMap<usize, ComputedStyle> = HashMap::new();
    compute_styles_recursive(&html, &sheets, None, &mut styles1);
    let mut engine = LayoutEngine::new();
    let mut body_root = build_layout_tree(&body, &styles1, Some(&mut engine.text_measurer));
    engine.layout(&mut body_root, viewport);

    let footer_y_full = find_by_class(&body_root, "footer").unwrap().rect.y;
    let todo_h_full = find_by_id(&body_root, "todo-list").unwrap().rect.height;

    eprintln!(
        "[diag runtime test] FULL layout (body root): footer.y={:.1} todo_list.h={:.1}",
        footer_y_full, todo_h_full
    );

    // ★ 关键：清除所有脏标记（模拟运行时 relayout 末尾的 clear_all_dirty）
    fn clear_dirty(n: &Rc<RefCell<Node>>) {
        n.borrow().mark_dirty(DirtyType::None);
        for child in &n.borrow().child_nodes() {
            clear_dirty(child);
        }
    }
    clear_dirty(&html);

    // ===== 第二轮：模拟输入后 partial_layout =====
    // ★ 关键：仅 input 被标记为脏（与运行时 handle_keyboard → set_text_content 一致）
    todo_input.borrow_mut().set_text_content("hello");

    // ★ 收集脏节点（从 html 开始，与运行时 window.rs 第 475 行一致）
    let (dirty_nodes, _total) = {
        let mut dirty = Vec::new();
        let mut total = 0;
        fn collect(n: &Rc<RefCell<Node>>, dirty: &mut Vec<Rc<RefCell<Node>>>, total: &mut usize) {
            *total += 1;
            if n.borrow().is_dirty() {
                dirty.push(n.clone());
            }
            for child in &n.borrow().child_nodes() {
                collect(child, dirty, total);
            }
        }
        collect(&html, &mut dirty, &mut total);
        (dirty, total)
    };

    // ★ 关键：使用 &body 作为根节点重建布局树
    let mut styles2: HashMap<usize, ComputedStyle> = HashMap::new();
    compute_styles_recursive(&html, &sheets, None, &mut styles2);
    let mut body_root2 = build_layout_tree(&body, &styles2, Some(&mut engine.text_measurer));

    eprintln!("[diag runtime test] dirty nodes: {}", dirty_nodes.len());

    // ★ 关键：使用 partial_layout（与运行时 window.rs 第 512 行一致）
    engine.partial_layout(&mut body_root2, &dirty_nodes, viewport, false, false);

    let footer_y_partial = find_by_class(&body_root2, "footer").unwrap().rect.y;
    let todo_h_partial = find_by_id(&body_root2, "todo-list").unwrap().rect.height;

    eprintln!(
        "[diag runtime test] PARTIAL layout (body root): footer.y={:.1} todo_list.h={:.1}",
        footer_y_partial, todo_h_partial
    );

    // ===== 断言 =====
    assert!(
        (todo_h_full - todo_h_partial).abs() < 2.0,
        "todo_list height should NOT change: full={:.1} partial={:.1}",
        todo_h_full, todo_h_partial
    );

    assert!(
        (footer_y_full - footer_y_partial).abs() < 1.0,
        "footer y should be STABLE (body root): full={:.1} partial={:.1}",
        footer_y_full, footer_y_partial
    );

    eprintln!(
        "[diag runtime test] PASS: footer stable at y={:.1} with body root",
        footer_y_partial
    );
}
