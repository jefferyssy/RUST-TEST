//! 编译管线 —— 将 HTML/CSS/JS 源文件编译为 Rust 代码。
//!
//! 提供两层 API：
//!
//! # 文件级 API（直接指定路径）
//!
//! | 函数 | 说明 |
//! |------|------|
//! | [`compile_body`] | 编译为 `main()` 函数体（不含 `fn main` 包装） |
//! | [`compile_body_with_options`] | 同上，可指定窗口参数 |
//! | [`compile`] | 编译为完整 `main.rs`（含 `fn main` 包装 + use 语句） |
//! | [`compile_with_options`] | 同上，可指定窗口参数 |
//! | [`compile_to_file`] | 编译完整 `main.rs` 并写入文件 |
//! | [`compile_body_to_file`] | 编译函数体并写入文件（for `include!`） |
//!
//! # 编译管线（Phase 0 — 5）
//!
//! ```text
//! Phase 1: html::parse_html()        → 元素树（HtmlElement）
//! Phase 2: variable::assign_*()      → 变量名分配
//! Phase 3: css::parse_css()          → CSS 规则匹配
//! Phase 4: js::compile_js()          → 事件处理器 + 共享状态
//! Phase 5: codegen::generate_*()     → Rust 代码输出
//! ```

use crate::js::{EventHandler, SharedStateVar};
use crate::variable;
use crate::{css, html, js};
use std::fs;

// ── 函数体级 API ──

/// 编译为 `main()` 函数体（默认窗口 800×600, title="Demo"）。
pub fn compile_body(html_path: &str, css_path: &str, js_path: &str) -> String {
    compile_body_with_options(html_path, css_path, js_path, "Demo", 800, 600)
}

/// 编译为 `main()` 函数体（自定义窗口参数）。
///
/// 从 HTML 中自动发现 CSS/JS；额外的 `css_path`/`js_path` 会合并进去（向后兼容）。
pub fn compile_body_with_options(
    html_path: &str,
    css_path: &str,
    js_path: &str,
    title: &str,
    width: u32,
    height: u32,
) -> String {
    use std::path::Path;

    let html_src = fs::read_to_string(html_path)
        .unwrap_or_else(|e| panic!("Cannot read HTML file '{}': {}", html_path, e));
    let input_dir = Path::new(html_path).parent().unwrap_or(Path::new("."));

    let mut resources = html::parse_html_document(&html_src, input_dir);

    // 向后兼容：额外指定的 CSS/JS 文件也加入来源
    if !css_path.is_empty() {
        resources.css_sources.push(html::CssSource::File(Path::new(css_path).to_path_buf()));
    }
    if !js_path.is_empty() {
        resources.js_sources.push(html::JsSource::File(Path::new(js_path).to_path_buf()));
    }

    compile_resources(&resources, title, width, height)
}

/// 核心管线：从已解析的 [`HtmlResources`] 生成 Rust 代码（单文件模式）。
pub(crate) fn compile_resources(
    resources: &html::HtmlResources,
    title: &str,
    width: u32,
    height: u32,
) -> String {
    let elements = &resources.elements;
    let root_assignments = variable::assign_root_variable_names(elements);
    let all_element_vars = variable::build_all_element_vars(elements);

    let css_rules = css::parse_css_sources(&resources.css_sources);
    let matched_styles = css::match_css_to_elements(&css_rules, elements, &all_element_vars);

    let mut handlers = Vec::new();
    let mut shared_state = Vec::new();
    for js_src in &resources.js_sources {
        let js_content = match js_src {
            html::JsSource::File(path) => {
                fs::read_to_string(path).unwrap_or_else(|e| panic!("Cannot read JS '{}': {e}", path.display()))
            }
            html::JsSource::Inline(content) => content.clone(),
        };
        let (h, s) = js::compile_js(&js_content, &all_element_vars, &css_rules);
        handlers.extend(h);
        shared_state.extend(s);
    }

    crate::codegen::CodeGenerator::generate_main_body(
        &root_assignments,
        &matched_styles,
        &handlers,
    )
}

/// 核心管线（拆分模式）：返回结构化数据用于多文件输出。
///
/// 每个 CSS/JS 源文件独立编译，一一对应输出 .rs 文件。
pub(crate) fn compile_resources_split(
    resources: &html::HtmlResources,
    title: &str,
    width: u32,
    height: u32,
) -> crate::codegen::MultiFileOutput {
    let elements = &resources.elements;
    let root_assignments = variable::assign_root_variable_names(elements);
    let all_element_vars = variable::build_all_element_vars(elements);

    // 逐 CSS 文件解析样式并匹配
    let mut css_files: Vec<(String, Vec<(String, String)>)> = Vec::new();
    for src in &resources.css_sources {
        let (name, content) = match src {
            html::CssSource::File(path) => (
                path.file_stem().unwrap_or_default().to_string_lossy().to_string(),
                fs::read_to_string(path).unwrap_or_default(),
            ),
            html::CssSource::Inline(content) => ("inline".into(), content.clone()),
            html::CssSource::InlineAttr { var_name: _, content } => ("inline_attr".into(), content.clone()),
        };
        let rules = css::parse_css(&content);
        let matched = css::match_css_to_elements(&rules, elements, &all_element_vars);
        css_files.push((name, matched));
    }

    // 逐 JS 文件编译
    let mut js_files: Vec<(String, Vec<EventHandler>, Vec<SharedStateVar>)> = Vec::new();
    for src in &resources.js_sources {
        let (name, js_content) = match src {
            html::JsSource::File(path) => (
                path.file_stem().unwrap_or_default().to_string_lossy().to_string(),
                fs::read_to_string(path).unwrap_or_else(|e| panic!("Cannot read JS '{}': {e}", path.display())),
            ),
            html::JsSource::Inline(content) => ("inline".into(), content.clone()),
        };
        let (h, s) = js::compile_js(&js_content, &all_element_vars, &css::parse_css_sources(&resources.css_sources));
        js_files.push((name, h, s));
    }

    // 从原始 CSS 规则中提取 body 样式（body 不在元素树中，需单独处理）
    let mut body_styles = String::new();
    for src in &resources.css_sources {
        let content = match src {
            html::CssSource::File(path) => fs::read_to_string(path).unwrap_or_default(),
            html::CssSource::Inline(c) => c.clone(),
            html::CssSource::InlineAttr { content, .. } => content.clone(),
        };
        for rule in css::parse_css(&content) {
            if rule.selector.trim() == "body" {
                let style_str = rule.declarations.iter()
                    .map(|(k, v)| format!("{}: {}", k, v))
                    .collect::<Vec<_>>()
                    .join("; ");
                if !style_str.is_empty() {
                    if !body_styles.is_empty() { body_styles.push_str("; "); }
                    body_styles.push_str(&style_str);
                }
            }
        }
    }

    crate::codegen::generate_multi_file(
        &root_assignments,
        &all_element_vars,
        &css_files,
        &js_files,
        &body_styles,
        title,
        width,
        height,
    )
}

// ── 完整文件级 API ──

/// 编译为完整 `main.rs`（默认窗口参数）。
pub fn compile(html_path: &str, css_path: &str, js_path: &str) -> String {
    compile_with_options(html_path, css_path, js_path, "Demo", 800, 600)
}

/// 编译为完整 `main.rs`（自定义窗口参数）。
pub fn compile_with_options(
    html_path: &str,
    css_path: &str,
    js_path: &str,
    title: &str,
    width: u32,
    height: u32,
) -> String {
    let body = compile_body_with_options(html_path, css_path, js_path, title, width, height);
    let mut output = String::new();
    output.push_str("//! Generated by rust-test compiler\n\n");
    output.push_str("use render_wgpu::WebWindow;\n\n");
    output.push_str("fn main() {\n");
    output.push_str(&body);
    output.push_str("}\n");
    output
}

// ── 文件输出 API ──

/// 编译为完整 `main.rs` 并直接写入指定路径。
///
/// 自动创建父目录（如不存在）。
///
/// # Panics
///
/// 写入失败时 panic。
pub fn compile_to_file(html_path: &str, css_path: &str, js_path: &str, output_path: &str) {
    let code = compile(html_path, css_path, js_path);
    if let Some(parent) = std::path::Path::new(output_path).parent() {
        fs::create_dir_all(parent).ok();
    }
    fs::write(output_path, &code)
        .unwrap_or_else(|e| panic!("Cannot write output '{}': {}", output_path, e));
}

/// 编译为函数体并写入文件（用于 `include!` 集成方式）。
///
/// 与 [`compile_to_file`] 的区别：输出不含 `fn main` 包装，
/// 由调用方的 `include!` 宏嵌入到自己的 `main()` 中。
///
/// # Panics
///
/// 写入失败时 panic。
pub fn compile_body_to_file(html_path: &str, css_path: &str, js_path: &str, output_path: &str) {
    let code = compile_body(html_path, css_path, js_path);
    if let Some(parent) = std::path::Path::new(output_path).parent() {
        fs::create_dir_all(parent).ok();
    }
    fs::write(output_path, &code)
        .unwrap_or_else(|e| panic!("Cannot write output '{}': {}", output_path, e));
}
