//! CSS 级联计算
//!
//! 负责将多个来源的 CSS 声明按优先级叠加以计算出最终样式。
//!
//! 级联排序规则（从低到高）：
//! 1. 用户代理样式（浏览器默认）
//! 2. 用户样式（Phase 2+）
//! 3. 作者样式（开发者定义的 CSS）
//! 4. 内联样式（style 属性）
//! 5. !important 规则
//!
//! Phase 3: 支持 match_selectors_full 以正确匹配组合器选择器

use std::collections::HashMap;
use std::cell::RefCell;
use std::rc::Rc;

use dom::ElementData;

use crate::selector::MatchedDeclaration;
use crate::stylesheet::{Declaration, StyleSheet};
use crate::values::{parse_css_value, CSSValue};
use crate::properties::parse_phase3_property;

/// 计算后的样式集合
/// 存储元素最终计算后的所有 CSS 属性值
#[derive(Debug, Clone)]
pub struct ComputedStyle {
    /// 属性名 → 计算后值 的映射
    pub properties: HashMap<String, CSSValue>,
}

impl ComputedStyle {
    /// 创建空的 ComputedStyle（全部使用初始值）
    pub fn new() -> Self {
        Self {
            properties: HashMap::new(),
        }
    }

    /// 获取属性值，不存在的属性返回 None
    pub fn get(&self, name: &str) -> Option<&CSSValue> {
        self.properties.get(name)
    }

    /// 获取属性值，不存在的属性返回 Initial
    pub fn get_or_initial(&self, name: &str) -> CSSValue {
        self.properties
            .get(name)
            .cloned()
            .unwrap_or(CSSValue::Initial)
    }

    /// 设置属性值
    pub fn set(&mut self, name: &str, value: CSSValue) {
        self.properties.insert(name.to_string(), value);
    }

    /// 合并另一个样式（低优先级：仅添加不存在的属性）
    pub fn merge(&mut self, other: &ComputedStyle) {
        for (prop, val) in &other.properties {
            self.properties.entry(prop.clone()).or_insert_with(|| val.clone());
        }
    }
}

/// 计算元素的最终样式
///
/// 参数：
///   - element: 目标元素
///   - parent_style: 父元素计算后样式（用于继承属性）
///   - stylesheets: 所有样式表
///   - inline_style: 内联样式声明（style 属性值）
///
/// 返回：属性名 → CSSValue 的映射
pub fn compute_element_style(
    element: &ElementData,
    _parent_style: Option<&ComputedStyle>,
    stylesheets: &[StyleSheet],
    inline_style: &[Declaration],
) -> ComputedStyle {
    // 1. 收集所有匹配的声明
    let mut matched = crate::selector::match_selectors(element, stylesheets);

    // 2. 将内联声明加入（最高优先级，除 !important 外）
    // 使用 u32::MAX 特异性确保内联样式覆盖所有非 !important 的作者样式
    for decl in inline_style {
        matched.push(MatchedDeclaration {
            specificity: (u32::MAX, u32::MAX, u32::MAX),
            declaration: decl.clone(),
        });
    }

    // 3. 按特异性排序（低 → 高），内联样式在最后
    matched.sort_by(|a, b| {
        if a.declaration.important && !b.declaration.important {
            return std::cmp::Ordering::Greater;
        }
        if !a.declaration.important && b.declaration.important {
            return std::cmp::Ordering::Less;
        }
        a.specificity.cmp(&b.specificity)
    });

    // 4. 依次应用到 ComputedStyle（后应用的覆盖先应用的）
    let mut style = ComputedStyle::new();
    for m in &matched {
        let value = parse_css_value(&m.declaration.property, &m.declaration.value);
        style.set(&m.declaration.property, value);
    }

    // 5. 继承父元素样式（仅继承属性）
    if let Some(parent) = _parent_style {
        apply_inherited(parent, &mut style);
    }

    // 6. 应用用户代理默认样式（最低优先级）
    apply_user_agent_defaults(element, &mut style);

    style
}

/// Phase 3: 带 Node 上下文计算样式（支持组合器选择器）
///
/// 与 compute_element_style 类似，但使用 match_selectors_full
/// 以便正确匹配使用了组合器的选择器（如 "div > span"）。
pub fn compute_element_style_with_node(
    node: &Rc<RefCell<dom::Node>>,
    parent_style: Option<&ComputedStyle>,
    stylesheets: &[StyleSheet],
    inline_style: &[Declaration],
) -> ComputedStyle {
    // 1. 收集所有匹配的声明（使用 Node 感知的匹配）
    let mut matched = crate::selector::match_selectors_full(node, stylesheets);

    // 2. 内联声明加入
    for decl in inline_style {
        matched.push(MatchedDeclaration {
            specificity: (u32::MAX, u32::MAX, u32::MAX),
            declaration: decl.clone(),
        });
    }

    // 3. 按特异性排序
    matched.sort_by(|a, b| {
        if a.declaration.important && !b.declaration.important {
            return std::cmp::Ordering::Greater;
        }
        if !a.declaration.important && b.declaration.important {
            return std::cmp::Ordering::Less;
        }
        a.specificity.cmp(&b.specificity)
    });

    // 4. 应用声明到 ComputedStyle
    let mut style = ComputedStyle::new();
    for m in &matched {
        // Phase 3: 先尝试使用新属性解析器
        let value = match parse_phase3_property(&m.declaration.property, &m.declaration.value) {
            Some(v) => v,
            None => parse_css_value(&m.declaration.property, &m.declaration.value),
        };
        style.set(&m.declaration.property, value);
    }

    // 5. 继承
    if let Some(parent) = parent_style {
        apply_inherited(parent, &mut style);
    }

    // 6. 应用用户代理默认样式
    if let dom::NodeType::Element(elem_data) = &node.borrow().node_type {
        apply_user_agent_defaults(elem_data, &mut style);
    }

    style
}

/// 应用继承属性：从父元素继承 CSS 属性
///
/// CSS 继承属性包括：color, font-*, line-height, text-align, white-space 等
/// Phase 0: 仅复制父元素中存在的且子元素未设置的属性
fn apply_inherited(parent: &ComputedStyle, child: &mut ComputedStyle) {
    // 从父元素继承 color
    if !child.properties.contains_key("color") {
        if let Some(val) = parent.properties.get("color") {
            child.properties.insert("color".to_string(), val.clone());
        }
    }
    // 从父元素继承 font-size
    if !child.properties.contains_key("font-size") {
        if let Some(val) = parent.properties.get("font-size") {
            child.properties.insert("font-size".to_string(), val.clone());
        }
    }
    // 从父元素继承 font-family
    if !child.properties.contains_key("font-family") {
        if let Some(val) = parent.properties.get("font-family") {
            child.properties.insert("font-family".to_string(), val.clone());
        }
    }
    // 从父元素继承 text-align
    if !child.properties.contains_key("text-align") {
        if let Some(val) = parent.properties.get("text-align") {
            child.properties.insert("text-align".to_string(), val.clone());
        }
    }
    // Phase 1+: 通过 properties.toml 的 inherited 标记自动判断
}

/// 用户代理默认样式：为 HTML 元素提供浏览器默认值
///
/// 优先级最低，仅当元素未通过 CSS/内联设置对应属性时生效。
fn apply_user_agent_defaults(element: &ElementData, style: &mut ComputedStyle) {
    let tag = element.tag_name();
    match tag.as_ref() {
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
            if !style.properties.contains_key("font-weight") {
                style.set("font-weight", CSSValue::Keyword("bold".into()));
            }
        }
        "strong" | "b" | "th" => {
            if !style.properties.contains_key("font-weight") {
                style.set("font-weight", CSSValue::Keyword("bold".into()));
            }
        }
        "em" | "i" | "cite" | "dfn" => {
            if !style.properties.contains_key("font-style") {
                style.set("font-style", CSSValue::Keyword("italic".into()));
            }
        }
        "a" | "link" => {
            if !style.properties.contains_key("color") {
                style.set("color", CSSValue::Color(dom::Color::rgb(0, 0, 238)));
            }
            if !style.properties.contains_key("text-decoration") {
                style.set("text-decoration", CSSValue::Keyword("underline".into()));
            }
        }
        "code" | "pre" | "kbd" | "samp" => {
            if !style.properties.contains_key("font-family") {
                style.set("font-family", CSSValue::Keyword("monospace".into()));
            }
        }
        _ => {}
    }
}

// Phase 1+: 级联缓存、initial/inherit/unset 处理

#[cfg(test)]
#[path = "cascade.test.rs"]
mod tests;
