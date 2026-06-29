//! 编译管线 —— 将 HTML/CSS/JS 源文件编译为 Rust 代码。
//!
//! # 编译管线
//!
//! ```text
//! Phase 1: html::parse_html()        → 元素树（HtmlElement）
//! Phase 2: variable::assign_*()      → 变量名分配
//! Phase 3: css::parse_css()          → CSS 规则
//! Phase 4: js::compile_js()          → 事件处理器 + 共享状态
//! Phase 5: codegen::generate_*()     → Rust 代码输出
//! ```

use crate::js::{EventHandler, SharedStateVar};
use crate::variable;
use crate::{css, html, js};
use std::fs;

/// CSS 模块规格（传给 codegen）。
pub(crate) struct CssModuleSpec {
    pub module_name: String,
    pub href: Option<String>,        // <link href>, inline 为 None
    pub id: Option<String>,          // <style id="..."> 或 <link id="...">
    pub is_inline_attr: bool,        // true = 来自 style="..." 属性，不生成 CssStyleSheet
    pub rules: Vec<css::CssRule>,
}

/// 核心管线：从已解析的 [`HtmlResources`] 生成 Rust 代码。
pub(crate) fn compile_resources(
    resources: &html::HtmlResources,
    html_stem: &str,
    title: &str,
    width: u32,
    height: u32,
    dev_view: bool,
) -> crate::codegen::MultiFileOutput {
    let elements = &resources.elements;

    // 逐 CSS 源文件解析为结构化规则
    let mut css_specs: Vec<CssModuleSpec> = Vec::new();
    let mut inline_counter = 0u32;

    for src in &resources.css_sources {
        let (module_name, content, href, id, is_inline_attr) = match src {
            html::CssSource::File { path, id } => {
                let stem = path.file_stem().unwrap_or_default().to_string_lossy();
                let ext = path.extension().unwrap_or_default().to_string_lossy();
                (
                    format!("{}_{}", stem, ext),
                    fs::read_to_string(path).unwrap_or_default(),
                    Some(path.to_string_lossy().to_string()),
                    id.clone(),
                    false,
                )
            }
            html::CssSource::Inline { content, id } => {
                let name = format!("__style_{}", inline_counter);
                inline_counter += 1;
                (name, content.clone(), None, id.clone(), false)
            }
            html::CssSource::InlineAttr { var_name: _, content } => {
                let name = format!("__attr_{}", inline_counter);
                inline_counter += 1;
                (name, content.clone(), None, None, true)
            }
        };
        let rules = css::parse_css(&content);
        css_specs.push(CssModuleSpec { module_name, href, id, is_inline_attr, rules });
    }

    // 逐 JS 文件编译
    let all_element_vars = variable::build_all_element_vars(elements);
    let mut js_handlers: Vec<(String, Vec<EventHandler>, Vec<SharedStateVar>)> = Vec::new();
    for src in &resources.js_sources {
        let (name, js_content) = match src {
            html::JsSource::File(path) => {
                let stem = path.file_stem().unwrap_or_default().to_string_lossy();
                let ext = path.extension().unwrap_or_default().to_string_lossy();
                (
                    format!("{}_{}", stem, ext),
                    fs::read_to_string(path).unwrap_or_else(|e| panic!("Cannot read JS '{}': {e}", path.display())),
                )
            }
            html::JsSource::Inline(content) => ("inline".into(), content.clone()),
        };
        let (h, s) = js::compile_js(&js_content, &all_element_vars, &css::parse_css_sources(&resources.css_sources));
        js_handlers.push((name, h, s));
    }

    let html_module_name = format!("{}_html", html_stem);

    crate::codegen::generate_multi_file(
        &html_module_name,
        elements,
        &css_specs,
        &js_handlers,
        title,
        width,
        height,
        dev_view,
    )
}
