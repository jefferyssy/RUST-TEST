//! 统一解析调度器
//!
//! Phase 1: 将 HTML/CSS/JS 解析统一调度，返回编译单元。
//! Phase 2+: 集成 html5ever, cssparser, swc。

use crate::css::CssRule;
use crate::html::HtmlElement;
use crate::js::EventHandler;

/// 编译单元 —— HTML + CSS + JS 解析后的中间表示
#[derive(Debug)]
pub struct CompilationUnit {
    pub elements: Vec<HtmlElement>,
    pub css_rules: Vec<CssRule>,
    pub event_handlers: Vec<EventHandler>,
    pub source_files: Vec<SourceFile>,
}

/// 源文件信息
#[derive(Debug, Clone)]
pub struct SourceFile {
    pub path: String,
    pub kind: SourceKind,
    pub content: String,
}

/// 源文件类型
#[derive(Debug, Clone, PartialEq)]
pub enum SourceKind {
    Html,
    Css,
    JavaScript,
}

/// 解析器 —— 统一调度 HTML/CSS/JS 解析
pub struct Parser;

impl Parser {
    /// 解析项目源文件为编译单元
    pub fn parse(html_path: &str, css_path: &str, js_path: &str) -> CompilationUnit {
        let html_src = std::fs::read_to_string(html_path)
            .unwrap_or_else(|e| panic!("Cannot read '{}': {}", html_path, e));
        let css_src = std::fs::read_to_string(css_path)
            .unwrap_or_else(|e| panic!("Cannot read '{}': {}", css_path, e));
        let js_src = std::fs::read_to_string(js_path)
            .unwrap_or_else(|e| panic!("Cannot read '{}': {}", js_path, e));

        let elements = crate::html::parse_html(&html_src);
        let css_rules = crate::css::parse_css(&css_src);

        let all_element_vars = crate::build_all_element_vars(&elements);
        let (handlers, _shared_state) = crate::js::compile_js(&js_src, &all_element_vars, &css_rules);

        CompilationUnit {
            elements,
            css_rules,
            event_handlers: handlers,
            source_files: vec![
                SourceFile { path: html_path.to_string(), kind: SourceKind::Html, content: html_src },
                SourceFile { path: css_path.to_string(), kind: SourceKind::Css, content: css_src },
                SourceFile { path: js_path.to_string(), kind: SourceKind::JavaScript, content: js_src },
            ],
        }
    }
}
