//! Rust 代码生成器
//!
//! 将编译单元生成为模块化的 Rust 代码：
//! - `html.rs` — DOM 元素构建 + HtmlElements 结构体
//! - `style_N.rs` — 每个 CSS 文件一个样式模块
//  - `handler_N.rs` — 每个 JS 文件一个事件处理器模块
//  - `main.rs` — 入口调度
//
//  向后兼容：CodeGenerator::generate_main_body 保留原单文件输出。

/// 代码生成器（向后兼容单文件模式）
pub struct CodeGenerator;

impl CodeGenerator {
    /// 生成 main() 函数体（单文件模式，保留给 pipeline 旧接口用）
    pub fn generate_main_body(
        assignments: &[VarAssignment],
        matched_styles: &[(String, String)],
        handlers: &[EventHandler],
    ) -> String {
        generate_main_body_single(assignments, matched_styles, handlers)
    }
}

use std::collections::HashMap;

use crate::html::HtmlElement;
use crate::js::{EventHandler, SharedStateVar};
use crate::variable::VarAssignment;

/// 多文件编译输出
pub struct MultiFileOutput {
    /// main.rs 内容
    pub main_rs: String,
    /// html.rs 内容
    pub html_rs: String,
    /// 样式文件：(模块名, 文件内容) 列表
    pub style_files: Vec<(String, String)>,
    /// 处理器文件：(模块名, 文件内容) 列表
    pub handler_files: Vec<(String, String)>,
}

/// 生成拆分后的多文件输出
///
/// `root_assignments`: 仅根元素（驱动 HTML 构建）
/// `all_element_vars`: 所有元素含子元素（用于 HtmlElements 结构体 + CSS/JS 引用）
pub fn generate_multi_file(
    root_assignments: &[VarAssignment],
    all_element_vars: &[(String, HtmlElement)],
    css_files: &[(String, Vec<(String, String)>)],
    js_files: &[(String, Vec<EventHandler>, Vec<SharedStateVar>)],
    body_style_str: &str,
    title: &str,
    width: u32,
    height: u32,
) -> MultiFileOutput {
    // 所有元素变量名 → HtmlElements 结构体
    let root_vars: Vec<String> = root_assignments.iter().map(|a| a.name.clone()).collect();
    let all_vars: Vec<String> = all_element_vars.iter().map(|(name, _)| name.clone()).collect();

    // 生成 html.rs
    let html_rs = generate_html_module(root_assignments, &root_vars, &all_vars);

    // 3. 生成 style_N.rs
    let style_files: Vec<(String, String)> = css_files
        .iter()
        .enumerate()
        .map(|(i, (_name, styles))| {
            let mod_name = format!("style_{}", i);
            let content = generate_style_module(&mod_name, styles);
            (mod_name, content)
        })
        .collect();

    // 4. 生成 handler_N.rs
    let handler_files: Vec<(String, String)> = js_files
        .iter()
        .enumerate()
        .map(|(i, (_name, handlers, state))| {
            let mod_name = format!("handler_{}", i);
            let content = generate_handler_module(&mod_name, handlers, state, &all_vars);
            (mod_name, content)
        })
        .collect();

    // 5. 生成 main.rs
    let main_rs = generate_main_module(
        title, width, height, body_style_str,
        &style_files, &handler_files,
    );

    MultiFileOutput {
        main_rs,
        html_rs,
        style_files,
        handler_files,
    }
}

// ═══════════════════════════════════════════════════════════
//  html.rs 生成
// ═══════════════════════════════════════════════════════════

/// `root_vars`: 仅根元素的变量名（挂载到 body）
/// `all_vars`:   所有元素的变量名（放入 HtmlElements 结构体）
fn generate_html_module(assignments: &[VarAssignment], root_vars: &[String], all_vars: &[String]) -> String {
    let mut code = String::new();
    code.push_str("//! 由 index.html 编译生成\n\n");
    code.push_str("use std::rc::Rc;\n");
    code.push_str("use std::cell::RefCell;\n");
    code.push_str("use dom::Node;\n\n");

    // ── HtmlElements 结构体（所有元素）──
    code.push_str("/// 所有 DOM 元素引用\n");
    code.push_str("pub struct HtmlElements {\n");
    for var in all_vars {
        code.push_str(&format!("    pub {}: Rc<RefCell<Node>>,\n", var));
    }
    code.push_str("}\n\n");

    // ── build_html 函数 ──
    code.push_str("/// 构建 DOM 树并返回元素引用\n");
    code.push_str("pub fn build_html(doc: &Rc<RefCell<dom::Document>>) -> HtmlElements {\n");

    let mut tag_counts: HashMap<String, usize> = HashMap::new();
    for assignment in assignments {
        code.push_str(&generate_element_code_inner(
            &assignment.element, &assignment.name, 1, &mut tag_counts,
        ));
        code.push('\n');
    }

    // 挂载到 body（仅根元素）
    code.push_str("    // ── 挂载到 body ──\n");
    for var in root_vars {
        code.push_str(&format!(
            "    doc.borrow().body().borrow_mut().append_child({}.clone());\n",
            var
        ));
    }
    code.push('\n');

    // 返回结构体（所有元素）
    code.push_str("    HtmlElements {\n");
    for var in all_vars {
        code.push_str(&format!("        {},\n", var));
    }
    code.push_str("    }\n");
    code.push_str("}\n");

    code
}

// ═══════════════════════════════════════════════════════════
//  style_N.rs 生成
// ═══════════════════════════════════════════════════════════

fn generate_style_module(mod_name: &str, matched_styles: &[(String, String)]) -> String {
    let mut code = String::new();
    code.push_str(&format!("//! {} — CSS 样式\n\n", mod_name));
    code.push_str("use crate::html::HtmlElements;\n\n");

    code.push_str("/// 应用样式到元素\n");
    code.push_str("pub fn apply_styles(elements: &HtmlElements) {\n");

    for (var_name, style_str) in matched_styles {
        let escaped = style_str.replace('\\', "\\\\").replace('"', "\\\"");
        code.push_str(&format!(
            "    elements.{}.borrow_mut().set_style(\"{}\");\n",
            var_name, escaped
        ));
    }

    code.push_str("}\n");
    code
}

// ═══════════════════════════════════════════════════════════
//  handler_N.rs 生成
// ═══════════════════════════════════════════════════════════

fn generate_handler_module(
    mod_name: &str,
    handlers: &[EventHandler],
    shared_state: &[SharedStateVar],
    element_vars: &[String],
) -> String {
    let mut code = String::new();
    code.push_str(&format!("//! {} — JS 事件处理器\n\n", mod_name));
    code.push_str("use std::rc::Rc;\n");
    code.push_str("use std::cell::RefCell;\n");
    code.push_str("use crate::html::HtmlElements;\n\n");

    code.push_str("/// 共享状态 + 事件处理器\n");
    code.push_str("pub fn setup_handlers(elements: &HtmlElements) {\n");

    // 共享状态
    if !shared_state.is_empty() {
        code.push_str("    // ── 共享状态 ──\n");
        for sv in shared_state {
            code.push_str(&format!(
                "    let {} = Rc::new(RefCell::new({}i32));\n",
                sv.name, sv.initial_value
            ));
        }
        code.push('\n');
    }

    // 事件处理器
    if !handlers.is_empty() {
        code.push_str("    // ── 事件绑定 ──\n");
        for handler in handlers {
            for cv in &handler.cloned_vars {
                let base = cv.trim_end_matches("_clone");
                // 如果 base 是元素变量名，使用 elements.base 引用
                if element_vars.contains(&base.to_string()) {
                    code.push_str(&format!("    let {} = elements.{}.clone();\n", cv, base));
                } else {
                    code.push_str(&format!("    let {} = {}.clone();\n", cv, base));
                }
            }
            for sv_name in &handler.shared_vars {
                code.push_str(&format!("    let {}_clone = {}.clone();\n", sv_name, sv_name));
            }

            code.push_str(&format!(
                "    elements.{}.borrow_mut().add_event_listener(\"{}\", Box::new(move |_: &dom::Event| {{\n",
                handler.element_var, handler.event_type
            ));
            for line in handler.body_code.lines() {
                code.push_str(&format!("        {}\n", line));
            }
            code.push_str("    }));\n\n");
        }
    }

    code.push_str("}\n");
    code
}

// ═══════════════════════════════════════════════════════════
//  main.rs 生成
// ═══════════════════════════════════════════════════════════

fn generate_main_module(
    title: &str,
    width: u32,
    height: u32,
    body_style_str: &str,
    style_files: &[(String, String)],
    handler_files: &[(String, String)],
) -> String {
    let escaped_title = title.replace('\\', "\\\\").replace('"', "\\\"");

    let mut code = String::new();
    code.push_str("//! Generated by rust-test compiler\n\n");
    code.push_str("use render_wgpu::WebWindow;\n\n");

    code.push_str("mod html;\n");
    for (name, _) in style_files {
        code.push_str(&format!("mod {};\n", name));
    }
    for (name, _) in handler_files {
        code.push_str(&format!("mod {};\n", name));
    }
    code.push('\n');

    code.push_str("fn main() {\n");
    code.push_str(&format!(
        "    let mut window = WebWindow::new(\"{}\", {}, {});\n",
        escaped_title, width, height
    ));
    code.push_str("    let doc = window.document();\n\n");

    // body 样式
    if !body_style_str.is_empty() {
        let escaped = body_style_str.replace('\\', "\\\\").replace('"', "\\\"");
        code.push_str(&format!(
            "    doc.borrow().body().borrow_mut().set_style(\"{}\");\n\n",
            escaped
        ));
    }

    code.push_str("    // ── 1. 构建 DOM 树 ──\n");
    code.push_str("    let elements = html::build_html(&doc);\n\n");

    if !style_files.is_empty() {
        code.push_str("    // ── 2. 应用样式 ──\n");
        for (name, _) in style_files {
            code.push_str(&format!("    {}::apply_styles(&elements);\n", name));
        }
        code.push('\n');
    }

    if !handler_files.is_empty() {
        code.push_str("    // ── 3. 注册事件处理器 ──\n");
        for (name, _) in handler_files {
            code.push_str(&format!("    {}::setup_handlers(&elements);\n", name));
        }
        code.push('\n');
    }

    code.push_str("    // ── 4. 启动渲染 ──\n");
    code.push_str("    window.run();\n");
    code.push_str("}\n");

    code
}

// ═══════════════════════════════════════════════════════════
//  元素代码生成（内部复用）
// ═══════════════════════════════════════════════════════════

fn generate_element_code_inner(
    el: &HtmlElement,
    var_name: &str,
    depth: usize,
    tag_counts: &mut HashMap<String, usize>,
) -> String {
    let indent = "    ".repeat(depth);
    let mut code = String::new();

    code.push_str(&format!("{}let {} = doc.borrow().create_element(\"{}\");\n", indent, var_name, el.tag));

    for (key, value) in &el.attributes {
        code.push_str(&format!("{}{}.borrow_mut().set_attribute(\"{}\", \"{}\");\n",
            indent, var_name, key, value.replace('\\', "\\\\").replace('"', "\\\"")));
    }

    if !el.text_content.is_empty() {
        let escaped = el.text_content.replace('\\', "\\\\").replace('"', "\\\"");
        code.push_str(&format!("{}{}.borrow_mut().set_text_content(\"{}\");\n", indent, var_name, escaped));
    }

    for child in &el.children {
        let child_name = crate::variable::generate_var_name(child, tag_counts);
        code.push_str(&generate_element_code_inner(child, &child_name, depth + 1, tag_counts));
        code.push_str(&format!("{}{}.borrow_mut().append_child({}.clone());\n", indent, var_name, child_name));
    }

    code
}

// ═══════════════════════════════════════════════════════════
//  向后兼容：单文件 main() 函数体生成
// ═══════════════════════════════════════════════════════════

fn generate_main_body_single(
    assignments: &[VarAssignment],
    matched_styles: &[(String, String)],
    handlers: &[EventHandler],
) -> String {
    let mut code = String::new();
    let mut tag_counts: HashMap<String, usize> = HashMap::new();

    code.push_str("    // ====== Compiled from index.html ======\n\n");
    let mut var_names: Vec<String> = Vec::new();
    for assignment in assignments {
        code.push_str(&generate_element_code_inner(
            &assignment.element, &assignment.name, 1, &mut tag_counts,
        ));
        code.push('\n');
        var_names.push(assignment.name.clone());
    }

    code.push_str("    // ====== Mount to body ======\n\n");
    for v in &var_names {
        code.push_str(&format!("    doc.borrow().body().borrow_mut().append_child({}.clone());\n", v));
    }
    code.push('\n');

    code.push_str("    // ====== Compiled from style.css ======\n\n");
    for (var_name, style_str) in matched_styles {
        let escaped = style_str.replace('\\', "\\\\").replace('"', "\\\"");
        code.push_str(&format!("    {}.borrow_mut().set_style(\"{}\");\n", var_name, escaped));
    }
    code.push('\n');

    code.push_str("    // ====== Compiled from app.js ======\n\n");
    for handler in handlers {
        for cv in &handler.cloned_vars {
            let base = cv.trim_end_matches("_clone");
            code.push_str(&format!("    let {} = {}.clone();\n", cv, base));
        }
        for sv_name in &handler.shared_vars {
            code.push_str(&format!("    let {}_clone = {}.clone();\n", sv_name, sv_name));
        }
        code.push_str(&format!(
            "    {}.borrow_mut().add_event_listener(\"{}\", Box::new(move |_: &dom::Event| {{\n",
            handler.element_var, handler.event_type
        ));
        for line in handler.body_code.lines() {
            code.push_str(&format!("        {}\n", line));
        }
        code.push_str("    }));\n\n");
    }

    code.push_str("    // ====== Start rendering ======\n\n");
    code.push_str("    window.run();\n");

    code
}
