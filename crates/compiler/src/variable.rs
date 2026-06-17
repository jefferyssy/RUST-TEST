//! 元素变量名分配
//!
//! 为 HTML 元素树中的每个节点分配 Rust 变量名，
//! 用于代码生成和 CSS/JS 匹配。

use crate::html::HtmlElement;
use std::collections::HashMap;

/// 分配给元素的变量名
#[derive(Debug, Clone)]
pub struct VarAssignment {
    pub name: String,
    pub element: HtmlElement,
}

/// 为顶层元素分配变量名（用于代码生成）
pub(crate) fn assign_root_variable_names(elements: &[HtmlElement]) -> Vec<VarAssignment> {
    let mut result = Vec::new();
    let mut tag_counts: HashMap<String, usize> = HashMap::new();
    for el in elements {
        let name = generate_var_name(el, &mut tag_counts);
        result.push(VarAssignment {
            name: name.clone(),
            element: el.clone(),
        });
    }
    result
}

/// 构建完整元素-变量名映射（含子元素，用于 CSS/JS 匹配）
pub(crate) fn build_all_element_vars(elements: &[HtmlElement]) -> Vec<(String, HtmlElement)> {
    let mut result = Vec::new();
    let mut tag_counts: HashMap<String, usize> = HashMap::new();
    collect_recursive(elements, &mut tag_counts, &mut result);
    result
}

/// 分配变量名（公开 API）
pub fn assign_variable_names(elements: &[HtmlElement]) -> Vec<VarAssignment> {
    let mut result = Vec::new();
    let mut tag_counts: HashMap<String, usize> = HashMap::new();
    collect_to_assignments(elements, &mut tag_counts, &mut result);
    result
}

/// 为单个元素生成变量名
///
/// 命名优先级：id > 第一个 class > tag 名
/// 重复 tag 名自动加数字后缀
pub fn generate_var_name(el: &HtmlElement, tag_counts: &mut HashMap<String, usize>) -> String {
    // 优先用 id
    if let Some(id) = el.attributes.get("id") {
        return id.replace('-', "_");
    }

    // 其次用第一个 class
    if let Some(class) = el.attributes.get("class") {
        if let Some(first_class) = class.split_whitespace().next() {
            return first_class.replace('-', "_");
        }
    }

    // 否则用 tag 名
    let count = tag_counts.entry(el.tag.clone()).or_insert(0);
    *count += 1;
    if *count == 1 {
        el.tag.clone()
    } else {
        format!("{}_{}", el.tag, count)
    }
}

/// 递归收集所有元素并分配变量名
fn collect_recursive(
    elements: &[HtmlElement],
    tag_counts: &mut HashMap<String, usize>,
    result: &mut Vec<(String, HtmlElement)>,
) {
    for el in elements {
        let name = generate_var_name(el, tag_counts);
        result.push((name.clone(), el.clone()));
        collect_recursive(&el.children, tag_counts, result);
    }
}

/// 递归收集元素到 VarAssignment 列表
fn collect_to_assignments(
    elements: &[HtmlElement],
    tag_counts: &mut HashMap<String, usize>,
    result: &mut Vec<VarAssignment>,
) {
    for el in elements {
        let name = generate_var_name(el, tag_counts);
        result.push(VarAssignment {
            name: name.clone(),
            element: el.clone(),
        });
        collect_to_assignments(&el.children, tag_counts, result);
    }
}
