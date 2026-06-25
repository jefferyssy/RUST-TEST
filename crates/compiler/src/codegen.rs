//! Rust 代码生成器
//!
//! VNode + DomRegistry + StyleEngine 架构。
//! 将编译单元生成为模块化的 Rust 代码：
//! - `html.rs` — VNode 树构造
//! - `style_N.rs` — CSS 样式表注册
//! - `handler_N.rs` — JS 事件处理器
//! - `main.rs` — 入口调度

use crate::html::HtmlElement;
use crate::js::{EventHandler, SharedStateVar};
use crate::pipeline::CssModuleSpec;

/// 多文件编译输出
pub struct MultiFileOutput {
    /// main.rs 内容
    pub main_rs: String,
    /// html.rs 内容（VNode 构造）
    pub html_rs: String,
    /// 样式文件：(stem, content) → stem.rs
    pub style_files: Vec<(String, String)>,
    /// 处理器文件：(stem, content) → stem.rs
    pub handler_files: Vec<(String, String)>,
}

// ═══════════════════════════════════════════════════════════
//  多文件代码生成
// ═══════════════════════════════════════════════════════════

/// 生成拆分后的多文件输出。
pub fn generate_multi_file(
    elements: &[HtmlElement],
    css_specs: &[CssModuleSpec],
    js_handlers: &[(String, Vec<EventHandler>, Vec<SharedStateVar>)],
    title: &str,
    width: u32,
    height: u32,
) -> MultiFileOutput {
    // html.rs — VNode 树构造
    let html_rs = generate_html_module(elements);

    // style_N.rs — 每个 CSS 源生成 create_style_sheet()
    let style_files: Vec<(String, String)> = css_specs
        .iter()
        .map(|spec| {
            let content = generate_style_module(spec);
            (spec.module_name.clone(), content)
        })
        .collect();

    // handler_N.rs — 事件处理器
    let handler_files: Vec<(String, String)> = js_handlers
        .iter()
        .map(|(stem, handlers, state)| {
            let content = generate_handler_module(stem, handlers, state, elements);
            (stem.clone(), content)
        })
        .collect();

    // main.rs — 入口调度
    let main_rs = generate_main_module(
        title, width, height,
        &style_files, &handler_files,
    );

    MultiFileOutput {
        main_rs,
        html_rs,
        style_files,
        handler_files,
    }
}

/// 生成 html.rs：VNode 树构造。
fn generate_html_module(elements: &[HtmlElement]) -> String {
    let mut code = String::new();
    code.push_str("//! 由 index.html 编译生成 — VNode 树\n\n");
    code.push_str("use dom_flat::*;\n\n");

    code.push_str("/// 构建 VNode 树\n");
    code.push_str("pub fn build_vnode() -> VNode {\n");

    let body_children: Vec<String> = elements
        .iter()
        .map(|el| html_element_to_vnode(el, 3))
        .collect();

    if body_children.is_empty() {
        code.push_str("    VElementVNode::new(\"body\").into()\n");
    } else {
        code.push_str("    VElementVNode::new(\"body\")\n");
        code.push_str("        .with_children(vec![\n");
        for child in &body_children {
            code.push_str(&format!("{},\n", child));
        }
        code.push_str("        ])\n");
        code.push_str("        .into()\n");
    }
    code.push_str("}\n");

    code
}

/// 递归将 HtmlElement 转为 builder 链式 VNode 表达式字符串（无临时变量）。
fn html_element_to_vnode(el: &HtmlElement, depth: usize) -> String {
    let indent = "    ".repeat(depth);
    let inner = "    ".repeat(depth + 1);

    // 收集子节点表达式
    let mut child_exprs: Vec<String> = Vec::new();
    if !el.text_content.is_empty() {
        child_exprs.push(format!(
            "{}    VNode::text(\"{}\")",
            inner,
            escape_rust_string(&el.text_content)
        ));
    }
    for child in &el.children {
        child_exprs.push(html_element_to_vnode(child, depth + 2));
    }

    let mut code = String::new();

    // 首行: VElementVNode::new("tag")
    code.push_str(&format!(
        "{}VElementVNode::new(\"{}\")",
        indent,
        el.tag.to_lowercase()
    ));

    // id
    if let Some(id_val) = el.attributes.get("id") {
        code.push_str(&format!(
            "\n{}.with_id(\"{}\")",
            inner,
            escape_rust_string(id_val)
        ));
    }

    // class(es)
    if let Some(class_val) = el.attributes.get("class") {
        let classes: Vec<String> = class_val
            .split_whitespace()
            .map(|c| format!("\"{}\"", escape_rust_string(c)))
            .collect();
        code.push_str(&format!(
            "\n{}.with_classes(vec![{}])",
            inner,
            classes.join(", ")
        ));
    }

    // style
    if let Some(style_val) = el.attributes.get("style") {
        code.push_str(&format!(
            "\n{}.with_style(\"{}\")",
            inner,
            escape_rust_string(style_val)
        ));
    }

    // 其他属性
    for (key, val) in &el.attributes {
        if key != "id" && key != "class" && key != "style" {
            code.push_str(&format!(
                "\n{}.with_attr(\"{}\", \"{}\")",
                inner,
                escape_rust_string(key),
                escape_rust_string(val)
            ));
        }
    }

    // 子节点
    if !child_exprs.is_empty() {
        code.push_str(&format!("\n{}.with_children(vec![", inner));
        for c in &child_exprs {
            code.push_str(&format!("\n{},", c));
        }
        code.push_str(&format!("\n{}])", inner));
    }

    code
}

/// 生成 style_N.rs：`create_style_sheet() -> CssStyleSheet`。
fn generate_style_module(spec: &CssModuleSpec) -> String {
    let stem = &spec.module_name;
    let rules = &spec.rules;
    let href_expr = match &spec.href {
        Some(h) => format!("Some(\"{}\".into())", escape_rust_string(h)),
        None => "None".into(),
    };

    // 按需导入
    let mut imports = vec!["CssStyleSheet", "CssRule", "Select"];
    let has_pseudo = rules.iter().any(|r| {
        let segs = crate::css::parse_selector(&r.selector);
        segs.iter().any(|s| !s.pseudo_classes.is_empty() || !s.pseudo_elements.is_empty())
    });
    if has_pseudo { imports.push("PseudoClass"); imports.push("PseudoElement"); }
    let has_attr = rules.iter().any(|r| {
        let segs = crate::css::parse_selector(&r.selector);
        segs.iter().any(|s| !s.attrs.is_empty())
    });
    if has_attr { imports.push("AttrOp"); }

    let mut code = String::new();
    code.push_str(&format!("//! {} — CSS 样式表\n\n", stem));
    code.push_str(&format!("use dom_flat::{{{}}};\n\n", imports.join(", ")));

    code.push_str("/// 创建 CSSStyleSheet（对应 `<link>` 或 `<style>`）。\n");
    code.push_str("pub fn create_style_sheet() -> CssStyleSheet {\n");
    code.push_str("    let rules = vec![\n");

    for rule in rules {
        let builder = selector_to_builder_expr(&rule.selector);
        if rule.declarations.is_empty() {
            code.push_str(&format!("        CssRule::on({}),\n", builder));
        } else {
            code.push_str(&format!("        CssRule::on({})\n", builder));
            for (prop, val) in &rule.declarations {
                code.push_str(&format!(
                    "            .decl(\"{}\", \"{}\")\n",
                    escape_rust_string(prop),
                    escape_rust_string(val)
                ));
            }
            code.push_str("        ,\n");
        }
    }

    code.push_str("    ];\n\n");
    code.push_str("    CssStyleSheet {\n");
    code.push_str("        sheetType: \"text/css\".into(),\n");
    code.push_str(&format!("        href: {},\n", href_expr));
    code.push_str("        title: None,\n");
    code.push_str("        media: \"all\".into(),\n");
    code.push_str("        disabled: false,\n");
    code.push_str("        cssRules: rules,\n");
    code.push_str("    }\n");
    code.push_str("}\n");

    code
}

/// 将 CssRule 列表重建为 CSS 文本（保留供后续切换）。
#[allow(dead_code)]
fn rules_to_css_text(rules: &[crate::css::CssRule]) -> String {
    let mut css = String::new();
    for rule in rules {
        css.push_str(&rule.selector);
        css.push_str(" {\n");
        for (prop, val) in &rule.declarations {
            css.push_str(&format!("    {}: {};\n", prop, val));
        }
        css.push_str("}\n\n");
    }
    css
}

/// 将选择器字符串转为 Select builder 表达式。
fn selector_to_builder_expr(selector: &str) -> String {
    let segs = crate::css::parse_selector(selector);
    if segs.is_empty() { return String::from("Select::tag(\"div\")"); }

    let mut expr = String::new();
    for (i, seg) in segs.iter().enumerate() {
        // 组合器（首段跳过）
        if i > 0 {
            match seg.combinator {
                Some("child") => expr.push_str(".child()"),
                Some("adjacent") => expr.push_str(".adjacent()"),
                Some("sibling") => expr.push_str(".sibling()"),
                _ => expr.push_str(".descendant()"),
            }
        }

        // 段内基本选择器
        let is_first_segment = i == 0;
        let mut need_and = false;
        if let Some(ref tag) = seg.tag {
            if is_first_segment && !need_and {
                expr.push_str(&format!("Select::tag(\"{}\")", escape_rust_string(tag)));
            } else {
                expr.push_str(&format!(".and_tag(\"{}\")", escape_rust_string(tag)));
            }
            need_and = true;
        }
        if let Some(ref id) = seg.id {
            if is_first_segment && !need_and {
                expr.push_str(&format!("Select::id(\"{}\")", escape_rust_string(id)));
            } else {
                expr.push_str(&format!(".and_id(\"{}\")", escape_rust_string(id)));
            }
            need_and = true;
        }
        for class in &seg.classes {
            if is_first_segment && !need_and {
                expr.push_str(&format!("Select::class(\"{}\")", escape_rust_string(class)));
            } else {
                expr.push_str(&format!(".and_class(\"{}\")", escape_rust_string(class)));
            }
            need_and = true;
        }
        for (name, op, val) in &seg.attrs {
            expr.push_str(&format!(".and_attr(\"{}\", AttrOp::{},\"{}\")",
                escape_rust_string(name), op, escape_rust_string(val)));
        }
        for pc in &seg.pseudo_classes {
            // 支持的伪类名直接映射
            let variant = match pc.as_str() {
                "hover" => "Hover", "active" => "Active", "focus" => "Focus",
                "first-child" => "FirstChild", "last-child" => "LastChild",
                "only-child" => "OnlyChild", "root" => "Root", "empty" => "Empty",
                "checked" => "Checked", "disabled" => "Disabled", "enabled" => "Enabled",
                "required" => "Required", "optional" => "Optional",
                "valid" => "Valid", "invalid" => "Invalid",
                "first-of-type" => "FirstOfType", "last-of-type" => "LastOfType",
                "only-of-type" => "OnlyOfType",
                _ => "Hover", // 回退
            };
            expr.push_str(&format!(".and_pseudo(PseudoClass::{})", variant));
        }
        for pe in &seg.pseudo_elements {
            let variant = match pe.as_str() {
                "before" => "Before", "after" => "After",
                "first-line" => "FirstLine", "first-letter" => "FirstLetter",
                "marker" => "Marker", "placeholder" => "Placeholder",
                "selection" => "Selection", "backdrop" => "Backdrop",
                _ => "Before",
            };
            expr.push_str(&format!(".and_pseudo_element(PseudoElement::{})", variant));
        }
    }
    expr
}

/// 生成 handler.rs：使用 document.querySelector 定位节点的事件处理器。
fn generate_handler_module(
    stem: &str,
    handlers: &[EventHandler],
    shared_state: &[SharedStateVar],
    elements: &[HtmlElement],
) -> String {
    let mut code = String::new();
    code.push_str(&format!("//! {} — JS 事件处理器\n\n", stem));
    code.push_str("use std::rc::Rc;\n");
    code.push_str("use std::cell::RefCell;\n");
    code.push_str("use dom_flat::*;\n\n");

    code.push_str("/// 共享状态 + 事件处理器\n");
    code.push_str("pub fn setup_handlers(document: &DomRegistry) {\n");

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
            // 通过 document.querySelector 定位目标元素
            let selector = find_element_selector(elements, &handler.element_var);
            let find_code = format!(
                "    let {} = document.querySelector(\"{}\")",
                handler.element_var, selector
            );
            code.push_str(&format!(
                "{} .expect(\"找不到元素: {}\");\n",
                find_code, selector
            ));

            // 克隆/查找外部变量
            // NodeId 元素通过 querySelector 定位（Copy 类型无需 clone），
            // 共享状态（Rc<RefCell<i32>>）仍需 clone
            let mut element_replacements: Vec<(String, String)> = Vec::new(); // (old_name, new_name)
            for cv in &handler.cloned_vars {
                let base = cv.trim_end_matches("_clone");
                if is_element_var(elements, base) {
                    let el_selector = find_element_selector(elements, base);
                    // 元素变量：去掉 _clone 后缀，NodeId 无需 clone
                    code.push_str(&format!(
                        "    let {} = document.querySelector(\"{}\")",
                        base, el_selector
                    ));
                    code.push_str(&format!(
                        " .expect(\"找不到元素: {}\");\n", el_selector
                    ));
                    element_replacements.push((cv.clone(), base.to_string()));
                } else {
                    code.push_str(&format!("    let {} = {}.clone();\n", cv, base));
                }
            }
            for sv_name in &handler.shared_vars {
                code.push_str(&format!("    let {}_clone = {}.clone();\n", sv_name, sv_name));
            }

            // 事件监听器
            code.push_str(&format!(
                "    {}.addEventListener(document, \"{}\", Box::new(move |_: &dom_flat::Event| {{\n",
                handler.element_var, handler.event_type
            ));
            for line in handler.body_code.lines() {
                let mut translated = translate_js_body_line(line, elements);
                // 替换 body 中的 _clone 变量名为裸变量名
                for (old_name, new_name) in &element_replacements {
                    translated = translated.replace(old_name, new_name);
                }
                code.push_str(&format!("        {}\n", translated));
            }
            code.push_str("    }));\n\n");
        }
    }

    code.push_str("}\n");
    code
}

/// 生成 main.rs：使用 DomRegistry + StyleEngine + document.styleSheets。
fn generate_main_module(
    title: &str,
    width: u32,
    height: u32,
    style_files: &[(String, String)],
    handler_files: &[(String, String)],
) -> String {
    let escaped_title = escape_rust_string(title);

    let mut code = String::new();
    code.push_str("//! Generated by rust-test compiler\n\n");
    code.push_str("use dom_flat::*;\n");
    code.push_str("use style_runtime::StyleEngine;\n\n");

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
        "    println!(\"Ruft App: {} ({}x{})\");\n",
        escaped_title, width, height
    ));
    code.push('\n');
    code.push_str("    let mut document = DomRegistry::new();\n");
    code.push_str("    let mut style_engine = StyleEngine::new();\n\n");

    // 1. 加载 DOM
    code.push_str("    // ── 1. 加载 DOM 树 ──\n");
    code.push_str("    document.loadVnode(html::build_vnode());\n\n");

    // 2. 加载样式表
    if !style_files.is_empty() {
        code.push_str("    // ── 2. 注册样式表到 document.styleSheets ──\n");
        for (name, _) in style_files {
            code.push_str(&format!("    document.addStyleSheet({}::create_style_sheet());\n", name));
        }
        code.push('\n');
    }

    // 3. 启动引擎，on_ready 回调注册事件处理器
    code.push_str("    // ── 3. 启动：计算样式 → 渲染 → 注册 JS ──\n");
    code.push_str("    style_engine.run(&mut document, |doc| {\n");
    if !handler_files.is_empty() {
        for (name, _) in handler_files {
            code.push_str(&format!("        {}::setup_handlers(doc);\n", name));
        }
    }
    code.push_str("    });\n");

    code.push_str("}\n");

    code
}

/// 在元素树中查找元素对应的 CSS 选择器字符串。
fn find_element_selector(elements: &[HtmlElement], var_name: &str) -> String {
    find_element_selector_recursive(elements, var_name)
        .unwrap_or_else(|| var_name.to_string())
}

fn find_element_selector_recursive(elements: &[HtmlElement], var_name: &str) -> Option<String> {
    for el in elements {
        // 按 id 匹配
        if let Some(id) = el.attributes.get("id") {
            let el_var = id.replace('-', "_");
            if el_var == var_name {
                return Some(format!("#{}", id));
            }
        }
        // 按第一 class 匹配
        if let Some(class) = el.attributes.get("class") {
            let first_class = class.split_whitespace().next().unwrap_or("");
            let el_var = first_class.replace('-', "_");
            if el_var == var_name {
                return Some(format!(".{}", first_class));
            }
        }
        // 按 tag 匹配
        let tag_var = el.tag.to_lowercase();
        if tag_var == var_name {
            return Some(tag_var);
        }

        // 递归子元素
        if let Some(found) = find_element_selector_recursive(&el.children, var_name) {
            return Some(found);
        }
    }
    None
}

/// 检查变量名是否对应元素（而非共享状态变量）。
fn is_element_var(elements: &[HtmlElement], var_name: &str) -> bool {
    is_element_var_recursive(elements, var_name)
}

fn is_element_var_recursive(elements: &[HtmlElement], var_name: &str) -> bool {
    for el in elements {
        if let Some(id) = el.attributes.get("id") {
            if id.replace('-', "_") == var_name {
                return true;
            }
        }
        if let Some(class) = el.attributes.get("class") {
            if let Some(first_class) = class.split_whitespace().next() {
                if first_class.replace('-', "_") == var_name {
                    return true;
                }
            }
        }
        if el.tag.to_lowercase() == var_name {
            return true;
        }
        if is_element_var_recursive(&el.children, var_name) {
            return true;
        }
    }
    false
}

/// 翻译 JS 编译器生成的旧 DOM API 调用 → 一期简化为 println! 输出。
fn translate_js_body_line(line: &str, _elements: &[HtmlElement]) -> String {
    let trimmed = line.trim();

    // 跳过注释
    if trimmed.starts_with("//") || trimmed.is_empty() {
        return line.to_string();
    }

    // 模式: xxx_clone.borrow_mut().set_text_content(&yyy.borrow().to_string());
    // 转换: println!("[DOM] {:?} setTextContent: {:?}", xxx, *yyy.borrow());
    if trimmed.contains(".borrow_mut().set_text_content(") {
        // 提取元素变量名（set_text_content 前，去除 _clone 后缀）
        let el_part = trimmed
            .split(".borrow_mut().set_text_content(")
            .next()
            .unwrap_or("???");
        // 提取内容表达式（括号内的内容）
        let content = trimmed
            .split(".borrow_mut().set_text_content(")
            .nth(1)
            .unwrap_or("???)");
        // 去掉尾部 ); 和 &
        let content = content
            .trim()
            .trim_end_matches(");")
            .trim_start_matches('&');
        // 转换 .borrow().to_string() → .borrow()
        let content = content
            .replace(".borrow().to_string()", ".borrow()");
        let indent = &line[..line.len() - trimmed.len()];
        // 使用实际变量名（如 display），而非字符串字面量，避免 unused variable 警告
        let el_var = el_part.trim().trim_end_matches("_clone");
        return format!(
            "{}println!(\"[DOM] {{:?}} setTextContent: {{:?}}\", {}, *{});",
            indent,
            el_var,
            content.trim(),
        );
    }

    line.to_string()
}

/// 转义字符串用于 Rust 字面量。
fn escape_rust_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}
